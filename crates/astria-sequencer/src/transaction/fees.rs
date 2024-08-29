use std::collections::HashMap;

use astria_core::{
    primitive::v1::{
        asset::{
            self,
            IbcPrefixed,
        },
        RollupId,
    },
    protocol::{
        genesis::v1alpha1::Fees,
        transaction::v1alpha1::{
            action::{
                Action,
                BridgeLockAction,
                FeeAssetChangeAction,
                FeeChange,
                FeeChangeAction,
            },
            UnsignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::eyre::{
    eyre,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tendermint::abci::{
    Event,
    EventAttributeIndexExt as _,
};
use tracing::{
    instrument,
    warn,
};

use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt,
    },
    assets::StateReadExt as _,
    authority::StateReadExt,
    bridge::StateReadExt as _,
    ibc::{
        ics20_withdrawal::establish_withdrawal_target,
        StateReadExt as _,
    },
    sequence::StateReadExt as _,
    transaction::{
        checks::PaymentMap,
        StateReadExt as _,
    },
};

#[derive(Debug)]
pub(crate) struct FeeInfo {
    pub(crate) amt: u128,
    pub(crate) events: Vec<Event>,
}

struct PaymentInfo<'a> {
    from: Option<[u8; 20]>,
    to: Option<[u8; 20]>,
    allowed_fee_assets: &'a mut Vec<IbcPrefixed>,
    fee_asset: asset::Denom,
    fees_by_asset: &'a mut HashMap<asset::IbcPrefixed, u128>,
    fee_payment_map: &'a mut HashMap<([u8; 20], [u8; 20], asset::Denom), FeeInfo>,
}

#[instrument(skip_all, err)]
pub(crate) async fn get_and_report_tx_fees<S: StateRead>(
    tx: &UnsignedTransaction,
    state: &S,
    return_payment_map: bool,
) -> Result<(HashMap<asset::IbcPrefixed, u128>, Option<PaymentMap>)> {
    let mut current_fees = get_fees_from_state(state)
        .await
        .wrap_err("failed to get fees from state")?;
    let mut to;
    let from;
    if return_payment_map {
        to = Some(
            state
                .get_sudo_address()
                .await
                .wrap_err("failed to get sudo address")?,
        );
        from = Some(
            state
                .get_current_source()
                .ok_or_eyre("failed to get payer address")?
                .address_bytes(),
        );
    } else {
        to = None;
        from = None;
    }
    let mut allowed_fee_assets = state
        .get_allowed_fee_assets()
        .await
        .wrap_err("failed to get allowed fee assets")?;

    let mut action_index = 0;
    let mut fees_by_asset = HashMap::new();
    let mut fee_payment_map = HashMap::new();
    for action in &tx.actions {
        match action {
            Action::Transfer(act) => transfer_update_fees(
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                current_fees.transfer_base_fee,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for transfer action")?,

            Action::Sequence(act) => sequence_update_fees(
                &current_fees,
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                &act.data,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for sequence action")?,

            Action::Ics20Withdrawal(act) => {
                if let Some(from) = from {
                    establish_withdrawal_target(act, state, from).await?;
                }
                ics20_withdrawal_updates_fees(
                    &mut PaymentInfo {
                        from,
                        to,
                        allowed_fee_assets: &mut allowed_fee_assets,
                        fee_asset: act.fee_asset.clone(),
                        fees_by_asset: &mut fees_by_asset,
                        fee_payment_map: &mut fee_payment_map,
                    },
                    current_fees.ics20_withdrawal_base_fee,
                    return_payment_map,
                    action_index,
                )
                .wrap_err("failed to increase transaction fees for ics20 withdrawal action")?;
            }

            Action::InitBridgeAccount(act) => init_bridge_account_update_fees(
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                current_fees.init_bridge_account_base_fee,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for init bridge account action")?,

            Action::BridgeLock(act) => bridge_lock_update_fees(
                act,
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                current_fees.transfer_base_fee,
                current_fees.bridge_lock_byte_cost_multiplier,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for bridge lock action")?,

            Action::BridgeUnlock(act) => bridge_unlock_update_fees(
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                current_fees.transfer_base_fee,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for bridge unlock action")?,

            Action::BridgeSudoChange(act) => bridge_sudo_change_update_fees(
                &mut PaymentInfo {
                    from,
                    to,
                    allowed_fee_assets: &mut allowed_fee_assets,
                    fee_asset: act.fee_asset.clone(),
                    fees_by_asset: &mut fees_by_asset,
                    fee_payment_map: &mut fee_payment_map,
                },
                current_fees.bridge_sudo_change_fee,
                return_payment_map,
                action_index,
            )
            .wrap_err("failed to increase transaction fees for bridge sudo change action")?,

            Action::SudoAddressChange(act) => to = Some(act.new_address.address_bytes()),
            Action::FeeAssetChange(act) => handle_fee_asset_change(act, &mut allowed_fee_assets),
            Action::FeeChange(act) => handle_fee_change(act, &mut current_fees),
            Action::ValidatorUpdate(_) | Action::Ibc(_) | Action::IbcRelayerChange(_) => {
                continue;
            }
        }
        action_index = action_index
            .checked_add(1)
            .expect("action index overflowed");
    }
    if return_payment_map {
        Ok((fees_by_asset, Some(fee_payment_map)))
    } else {
        Ok((fees_by_asset, None))
    }
}

/// Pays fees from from to to based on the fee payment map and records the fee events to the
/// state.
#[instrument(skip_all, err)]
pub(crate) async fn pay_fees<S: StateWrite>(
    state: &mut S,
    fee_payment_map: HashMap<([u8; 20], [u8; 20], asset::Denom), FeeInfo>,
) -> Result<()> {
    for ((from, to, fee_asset), fee_info) in &fee_payment_map {
        state
            .decrease_balance(*from, fee_asset, fee_info.amt)
            .await
            .wrap_err("failed to decrease from balance")?;
        state
            .increase_balance(*to, fee_asset, fee_info.amt)
            .await
            .wrap_err("failed to increase to balance")?;
        for event in &fee_info.events {
            state.record(event.clone());
        }
    }
    Ok(())
}

fn transfer_update_fees(
    payment_info: &mut PaymentInfo,
    transfer_base_fee: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(transfer_base_fee))
        .or_insert(transfer_base_fee);

    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            transfer_base_fee,
            fee_payment_map,
            "TransferAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn sequence_update_fees(
    fees: &Fees,
    payment_info: &mut PaymentInfo,
    data: &[u8],
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    let fee = crate::sequence::action::calculate_fee(
        data,
        fees.sequence_byte_cost_multiplier,
        fees.sequence_base_fee,
    )
    .ok_or_eyre("calculated fee overflows u128")?;
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(fee))
        .or_insert(fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            fee,
            fee_payment_map,
            "SequenceAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn ics20_withdrawal_updates_fees(
    payment_info: &mut PaymentInfo,
    ics20_withdrawal_base_fee: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(ics20_withdrawal_base_fee))
        .or_insert(ics20_withdrawal_base_fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            ics20_withdrawal_base_fee,
            fee_payment_map,
            "Ics20WithdrawalAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn init_bridge_account_update_fees(
    payment_info: &mut PaymentInfo,
    init_bridge_account_base_fee: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(init_bridge_account_base_fee))
        .or_insert(init_bridge_account_base_fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            init_bridge_account_base_fee,
            fee_payment_map,
            "InitBridgeAccountAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn bridge_lock_update_fees(
    act: &BridgeLockAction,
    payment_info: &mut PaymentInfo,
    transfer_base_fee: u128,
    bridge_lock_byte_cost_multiplier: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    let expected_deposit_fee = transfer_base_fee.saturating_add(
        crate::bridge::get_deposit_byte_len(&Deposit::new(
            act.to,
            // rollup ID doesn't matter here, as this is only used as a size-check
            RollupId::from_unhashed_bytes([0; 32]),
            act.amount,
            act.asset.clone(),
            act.destination_chain_address.clone(),
        ))
        .saturating_mul(bridge_lock_byte_cost_multiplier),
    );

    fees_by_asset
        .entry(act.asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(expected_deposit_fee))
        .or_insert(expected_deposit_fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            expected_deposit_fee,
            fee_payment_map,
            "BridgeLockAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(&act.fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn bridge_unlock_update_fees(
    payment_info: &mut PaymentInfo,
    transfer_base_fee: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(transfer_base_fee))
        .or_insert(transfer_base_fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            transfer_base_fee,
            fee_payment_map,
            "BridgeUnlockAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn bridge_sudo_change_update_fees(
    payment_info: &mut PaymentInfo,
    bridge_sudo_change_base_fee: u128,
    add_to_payment_map: bool,
    action_index: u32,
) -> Result<()> {
    let PaymentInfo {
        from,
        to,
        allowed_fee_assets,
        fee_asset,
        fees_by_asset,
        fee_payment_map,
    } = payment_info;

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(bridge_sudo_change_base_fee))
        .or_insert(bridge_sudo_change_base_fee);
    if add_to_payment_map {
        err_if_asset_type_not_allowed(fee_asset, allowed_fee_assets)?;
        increase_fees(
            from.expect("from address should be `Some`"),
            to.expect("to address should be `Some`"),
            fee_asset,
            bridge_sudo_change_base_fee,
            fee_payment_map,
            "BridgeSudoChangeAction".to_string(),
            action_index,
        );
    } else {
        warn_if_asset_type_not_allowed(fee_asset, allowed_fee_assets);
    }
    Ok(())
}

fn handle_fee_asset_change(act: &FeeAssetChangeAction, allowed_fee_assets: &mut Vec<IbcPrefixed>) {
    match act {
        FeeAssetChangeAction::Addition(asset) => {
            allowed_fee_assets.push(asset.clone().into());
        }
        FeeAssetChangeAction::Removal(asset) => {
            allowed_fee_assets.retain(|a| *a != IbcPrefixed::from(asset));
        }
    }
}

fn handle_fee_change(act: &FeeChangeAction, current_fees: &mut Fees) {
    match act.fee_change {
        FeeChange::TransferBaseFee => {
            current_fees.transfer_base_fee = act.new_value;
        }
        FeeChange::SequenceBaseFee => {
            current_fees.sequence_base_fee = act.new_value;
        }
        FeeChange::SequenceByteCostMultiplier => {
            current_fees.sequence_byte_cost_multiplier = act.new_value;
        }
        FeeChange::InitBridgeAccountBaseFee => {
            current_fees.init_bridge_account_base_fee = act.new_value;
        }
        FeeChange::BridgeLockByteCostMultiplier => {
            current_fees.bridge_lock_byte_cost_multiplier = act.new_value;
        }
        FeeChange::BridgeSudoChangeBaseFee => {
            current_fees.bridge_sudo_change_fee = act.new_value;
        }
        FeeChange::Ics20WithdrawalBaseFee => {
            current_fees.ics20_withdrawal_base_fee = act.new_value;
        }
    }
}

#[instrument(skip_all, err)]
async fn get_fees_from_state<S: StateRead>(state: &S) -> Result<Fees> {
    let transfer_base_fee = state
        .get_transfer_base_fee()
        .await
        .wrap_err("failed to get transfer base fee")?;
    let sequence_base_fee = state
        .get_sequence_action_base_fee()
        .await
        .wrap_err("failed to get base fee")?;
    let sequence_byte_cost_multiplier = state
        .get_sequence_action_byte_cost_multiplier()
        .await
        .wrap_err("failed to get fee per byte")?;
    let init_bridge_account_base_fee = state
        .get_init_bridge_account_base_fee()
        .await
        .wrap_err("failed to get init bridge account base fee")?;
    let bridge_lock_byte_cost_multiplier = state
        .get_bridge_lock_byte_cost_multiplier()
        .await
        .wrap_err("failed to get bridge lock byte cost multiplier")?;
    let bridge_sudo_change_fee = state
        .get_bridge_sudo_change_base_fee()
        .await
        .wrap_err("failed to get bridge sudo change fee")?;
    let ics20_withdrawal_base_fee = state
        .get_ics20_withdrawal_base_fee()
        .await
        .wrap_err("failed to get ics20 withdrawal base fee")?;
    Ok(Fees {
        transfer_base_fee,
        sequence_base_fee,
        sequence_byte_cost_multiplier,
        init_bridge_account_base_fee,
        bridge_lock_byte_cost_multiplier,
        bridge_sudo_change_fee,
        ics20_withdrawal_base_fee,
    })
}

/// Adds the fee amount to be paid by the from to the to along with an event of kind
/// "tx.fees" to the payment map
fn increase_fees(
    from: [u8; 20],
    to: [u8; 20],
    fee_asset: &asset::Denom,
    amount: u128,
    fee_payment_map: &mut HashMap<([u8; 20], [u8; 20], asset::Denom), FeeInfo>,
    action_type: String,
    action_index: u32,
) {
    let fee_event = construct_tx_fee_event(fee_asset, amount, action_type, action_index);
    fee_payment_map
        .entry((from, to, fee_asset.clone()))
        .and_modify(|fee_info| {
            fee_info.amt = fee_info.amt.saturating_add(amount);
            fee_info.events.push(fee_event.clone());
        })
        .or_insert(FeeInfo {
            amt: amount,
            events: vec![fee_event],
        });
}

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
pub(crate) fn construct_tx_fee_event<T: std::fmt::Display>(
    asset: &T,
    fee_amount: u128,
    action_type: String,
    action_index: u32,
) -> Event {
    Event::new(
        "tx.fees",
        [
            ("asset", asset.to_string()).index(),
            ("feeAmount", fee_amount.to_string()).index(),
            ("actionType", action_type).index(),
            ("actionIndex", action_index.to_string()).index(),
        ],
    )
}

#[instrument(skip_all)]
fn warn_if_asset_type_not_allowed(asset: &asset::Denom, allowed_fee_assets: &[IbcPrefixed]) {
    if !allowed_fee_assets.contains(&IbcPrefixed::from(asset)) {
        warn!("Transaction execution will fail, asset type not allowed for fee payment: {asset}");
    }
}

#[instrument(skip_all, err)]
fn err_if_asset_type_not_allowed(
    asset: &asset::Denom,
    allowed_fee_assets: &[IbcPrefixed],
) -> Result<()> {
    if allowed_fee_assets.contains(&IbcPrefixed::from(asset)) {
        Ok(())
    } else {
        Err(eyre!("asset type not allowed for fee payment: {asset}"))
    }
}
