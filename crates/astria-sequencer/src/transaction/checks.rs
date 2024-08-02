use std::collections::HashMap;

use anyhow::{
    ensure,
    Context as _,
};
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1alpha1::{
        action::{
            Action,
            BridgeLockAction,
        },
        SignedTransaction,
        UnsignedTransaction,
    },
};
use cnidarium::StateRead;
use tracing::instrument;

use crate::{
    accounts::StateReadExt as _,
    bridge::StateReadExt as _,
    ibc::StateReadExt as _,
};

#[instrument(skip_all)]
pub(crate) async fn get_fees_for_transaction<S: StateRead>(
    tx: &UnsignedTransaction,
    state: &S,
) -> anyhow::Result<HashMap<asset::IbcPrefixed, u128>> {
    let transfer_fee = state
        .get_transfer_base_fee()
        .await
        .context("failed to get transfer base fee")?;
    let ics20_withdrawal_fee = state
        .get_ics20_withdrawal_base_fee()
        .await
        .context("failed to get ics20 withdrawal base fee")?;
    let init_bridge_account_fee = state
        .get_init_bridge_account_base_fee()
        .await
        .context("failed to get init bridge account base fee")?;
    let bridge_lock_byte_cost_multiplier = state
        .get_bridge_lock_byte_cost_multiplier()
        .await
        .context("failed to get bridge lock byte cost multiplier")?;
    let bridge_sudo_change_fee = state
        .get_bridge_sudo_change_base_fee()
        .await
        .context("failed to get bridge sudo change fee")?;

    let mut fees_by_asset = HashMap::new();
    for action in &tx.actions {
        match action {
            Action::Transfer(act) => {
                transfer_update_fees(&act.fee_asset, &mut fees_by_asset, transfer_fee);
            }
            Action::Sequence(act) => {
                sequence_update_fees(state, &act.fee_asset, &mut fees_by_asset, &act.data).await?;
            }
            Action::Ics20Withdrawal(act) => ics20_withdrawal_updates_fees(
                &act.fee_asset,
                &mut fees_by_asset,
                ics20_withdrawal_fee,
            ),
            Action::InitBridgeAccount(act) => {
                fees_by_asset
                    .entry(act.fee_asset.to_ibc_prefixed())
                    .and_modify(|amt| *amt = amt.saturating_add(init_bridge_account_fee))
                    .or_insert(init_bridge_account_fee);
            }
            Action::BridgeLock(act) => bridge_lock_update_fees(
                act,
                &mut fees_by_asset,
                transfer_fee,
                bridge_lock_byte_cost_multiplier,
            ),
            Action::BridgeUnlock(act) => {
                bridge_unlock_update_fees(&act.fee_asset, &mut fees_by_asset, transfer_fee);
            }
            Action::BridgeSudoChange(act) => {
                fees_by_asset
                    .entry(act.fee_asset.to_ibc_prefixed())
                    .and_modify(|amt| *amt = amt.saturating_add(bridge_sudo_change_fee))
                    .or_insert(bridge_sudo_change_fee);
            }
            Action::ValidatorUpdate(_)
            | Action::SudoAddressChange(_)
            | Action::Ibc(_)
            | Action::IbcRelayerChange(_)
            | Action::FeeAssetChange(_)
            | Action::FeeChange(_) => {
                continue;
            }
        }
    }
    Ok(fees_by_asset)
}

// Checks that the account has enough balance to cover the total fees and transferred values
// for all actions in the transaction.
#[instrument(skip_all)]
pub(crate) async fn check_balance_for_total_fees_and_transfers<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let mut cost_by_asset = get_fees_for_transaction(tx.unsigned_transaction(), state)
        .await
        .context("failed to get fees for transaction")?;

    // add values transferred within the tx to the cost
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                cost_by_asset
                    .entry(act.asset.to_ibc_prefixed())
                    .and_modify(|amt| *amt = amt.saturating_add(act.amount))
                    .or_insert(act.amount);
            }
            Action::Ics20Withdrawal(act) => {
                cost_by_asset
                    .entry(act.denom.to_ibc_prefixed())
                    .and_modify(|amt| *amt = amt.saturating_add(act.amount))
                    .or_insert(act.amount);
            }
            Action::BridgeLock(act) => {
                cost_by_asset
                    .entry(act.asset.to_ibc_prefixed())
                    .and_modify(|amt| *amt = amt.saturating_add(act.amount))
                    .or_insert(act.amount);
            }
            Action::BridgeUnlock(act) => {
                let asset = state
                    .get_bridge_account_ibc_asset(tx)
                    .await
                    .context("failed to get bridge account asset id")?;
                cost_by_asset
                    .entry(asset)
                    .and_modify(|amt| *amt = amt.saturating_add(act.amount))
                    .or_insert(act.amount);
            }
            Action::ValidatorUpdate(_)
            | Action::SudoAddressChange(_)
            | Action::Sequence(_)
            | Action::InitBridgeAccount(_)
            | Action::BridgeSudoChange(_)
            | Action::Ibc(_)
            | Action::IbcRelayerChange(_)
            | Action::FeeAssetChange(_)
            | Action::FeeChange(_) => {
                continue;
            }
        }
    }

    for (asset, total_fee) in cost_by_asset {
        let balance = state
            .get_account_balance(tx, asset)
            .await
            .context("failed to get account balance")?;
        ensure!(
            balance >= total_fee,
            "insufficient funds for asset {}",
            asset
        );
    }

    Ok(())
}

fn transfer_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    transfer_fee: u128,
) {
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(transfer_fee))
        .or_insert(transfer_fee);
}

async fn sequence_update_fees<S: StateRead>(
    state: &S,
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    data: &[u8],
) -> anyhow::Result<()> {
    let fee = crate::sequence::calculate_fee_from_state(data, state)
        .await
        .context("fee for sequence action overflowed; data too large")?;
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(fee))
        .or_insert(fee);
    Ok(())
}

fn ics20_withdrawal_updates_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    ics20_withdrawal_fee: u128,
) {
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(ics20_withdrawal_fee))
        .or_insert(ics20_withdrawal_fee);
}

fn bridge_lock_update_fees(
    act: &BridgeLockAction,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    transfer_fee: u128,
    bridge_lock_byte_cost_multiplier: u128,
) {
    use astria_core::sequencerblock::v1alpha1::block::Deposit;

    let expected_deposit_fee = transfer_fee.saturating_add(
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
}

fn bridge_unlock_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    transfer_fee: u128,
) {
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(transfer_fee))
        .or_insert(transfer_fee);
}
