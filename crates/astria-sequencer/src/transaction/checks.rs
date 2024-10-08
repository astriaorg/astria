use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1alpha1::{
        action::{
            self,
            Action,
            BridgeLockAction,
            BridgeSudoChangeAction,
            BridgeUnlockAction,
            InitBridgeAccountAction,
            TransferAction,
        },
        SignedTransaction,
        UnsignedTransaction,
    },
};
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::StateRead;
use tracing::instrument;

use crate::{
    accounts::StateReadExt as _,
    address::StateReadExt as _,
    app::StateReadExt as _,
    bridge::StateReadExt as _,
    fees::{
        FeeHandler,
        GenericFeeComponents,
    },
};

#[instrument(skip_all)]
pub(crate) async fn check_nonce_mempool<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> Result<()> {
    let signer_address = state
        .try_base_prefixed(tx.verification_key().address_bytes())
        .await
        .wrap_err(
            "failed constructing the signer address from signed transaction verification and \
             prefix provided by app state",
        )?;
    let curr_nonce = state
        .get_account_nonce(&signer_address)
        .await
        .wrap_err("failed to get account nonce")?;
    ensure!(tx.nonce() >= curr_nonce, "nonce already used by account");
    Ok(())
}

#[instrument(skip_all)]
pub(crate) async fn check_chain_id_mempool<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> Result<()> {
    let chain_id = state
        .get_chain_id()
        .await
        .wrap_err("failed to get chain id")?;
    ensure!(tx.chain_id() == chain_id.as_str(), "chain id mismatch");
    Ok(())
}

#[instrument(skip_all)]
pub(crate) async fn get_fees_for_transaction<S: StateRead>(
    tx: &UnsignedTransaction,
    state: &S,
) -> Result<HashMap<asset::IbcPrefixed, u128>> {
    let transfer_fees = TransferAction::fee_components(state)
        .await
        .wrap_err("failed to get transfer fees")?;
    let ics20_withdrawal_fees = action::Ics20Withdrawal::fee_components(state)
        .await
        .wrap_err("failed to get ics20 withdrawal fees")?;
    let init_bridge_account_fees = InitBridgeAccountAction::fee_components(state)
        .await
        .wrap_err("failed to get init bridge account fees")?;
    let bridge_lock_fees = BridgeLockAction::fee_components(state)
        .await
        .wrap_err("failed to get bridge lock fees")?;
    let bridge_unlock_fees = BridgeUnlockAction::fee_components(state)
        .await
        .wrap_err("failed to get bridge unlock fees")?;
    let bridge_sudo_change_fees = BridgeSudoChangeAction::fee_components(state)
        .await
        .wrap_err("failed to get bridge sudo change fees")?;

    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                transfer_update_fees(&act.fee_asset, &mut fees_by_asset, &transfer_fees);
            }
            Action::Sequence(act) => {
                sequence_update_fees(state, &act.fee_asset, &mut fees_by_asset, &act.data).await?;
            }
            Action::Ics20Withdrawal(act) => ics20_withdrawal_updates_fees(
                &act.fee_asset,
                &mut fees_by_asset,
                &ics20_withdrawal_fees,
            ),
            Action::InitBridgeAccount(act) => {
                init_bridge_account_update_fees(
                    &act.fee_asset,
                    &mut fees_by_asset,
                    &init_bridge_account_fees,
                );
            }
            Action::BridgeLock(act) => {
                bridge_lock_update_fees(act, &mut fees_by_asset, &bridge_lock_fees);
            }
            Action::BridgeUnlock(act) => {
                bridge_unlock_update_fees(&act.fee_asset, &mut fees_by_asset, &bridge_unlock_fees);
            }
            Action::BridgeSudoChange(act) => {
                bridge_sudo_change_update_fees(
                    &act.fee_asset,
                    &mut fees_by_asset,
                    &bridge_sudo_change_fees,
                );
            }
            Action::ValidatorUpdate(_)
            | Action::SudoAddressChange(_)
            | Action::IbcSudoChange(_)
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
) -> Result<()> {
    let cost_by_asset = get_total_transaction_cost(tx, state)
        .await
        .context("failed to get transaction costs")?;

    for (asset, total_fee) in cost_by_asset {
        let balance = state
            .get_account_balance(&tx, &asset)
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

// Returns the total cost of the transaction (fees and transferred values for all actions in the
// transaction).
#[instrument(skip_all)]
pub(crate) async fn get_total_transaction_cost<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> Result<HashMap<asset::IbcPrefixed, u128>> {
    let mut cost_by_asset: HashMap<asset::IbcPrefixed, u128> =
        get_fees_for_transaction(tx.unsigned_transaction(), state)
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
                    .get_bridge_account_ibc_asset(&tx)
                    .await
                    .wrap_err("failed to get bridge account asset id")?;
                cost_by_asset
                    .entry(asset)
                    .and_modify(|amt| *amt = amt.saturating_add(act.amount))
                    .or_insert(act.amount);
            }
            Action::ValidatorUpdate(_)
            | Action::SudoAddressChange(_)
            | Action::IbcSudoChange(_)
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

    Ok(cost_by_asset)
}

fn transfer_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    transfer_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(transfer_fees, 0);
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

async fn sequence_update_fees<S: StateRead>(
    state: &S,
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    data: &[u8],
) -> Result<()> {
    let fee = crate::fees::calculate_sequence_action_fee_from_state(data, state)
        .await
        .wrap_err("fee for sequence action overflowed; data too large")?;
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(fee))
        .or_insert(fee);
    Ok(())
}

fn ics20_withdrawal_updates_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    ics20_withdrawal_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(ics20_withdrawal_fees, 0);
    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_lock_update_fees(
    act: &BridgeLockAction,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_lock_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(bridge_lock_fees, act.computed_cost_base_component());

    fees_by_asset
        .entry(act.asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn init_bridge_account_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    init_bridge_account_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(init_bridge_account_fees, 0);

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_unlock_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_lock_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(bridge_lock_fees, 0);

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_sudo_change_update_fees(
    fee_asset: &asset::Denom,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_sudo_change_fees: &Option<GenericFeeComponents>,
) {
    let total_fees = calculate_total_fees(bridge_sudo_change_fees, 0);

    fees_by_asset
        .entry(fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn calculate_total_fees(fees: &Option<GenericFeeComponents>, base_multiplier: u128) -> u128 {
    let (base_fee, multiplier) = match fees {
        Some(fee_components) => (
            fee_components.base_fee,
            fee_components.computed_cost_multiplier,
        ),
        None => (0, 0),
    };
    base_fee.saturating_add(base_multiplier.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use action::{
        BridgeLockFeeComponents,
        BridgeSudoChangeFeeComponents,
        BridgeUnlockFeeComponents,
        FeeComponents,
        Ics20WithdrawalFeeComponents,
        InitBridgeAccountFeeComponents,
        SequenceFeeComponents,
        TransferFeeComponents,
    };
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            ADDRESS_LEN,
        },
        protocol::transaction::v1alpha1::action::{
            SequenceAction,
            TransferAction,
        },
    };
    use bytes::Bytes;
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        address::{
            StateReadExt,
            StateWriteExt as _,
        },
        app::test_utils::*,
        assets::StateWriteExt as _,
        fees::{
            FeeHandler,
            StateWriteExt as _,
        },
        test_utils::ASTRIA_PREFIX,
    };

    #[tokio::test]
    #[expect(clippy::too_many_lines, reason = "it's a test")]
    async fn check_balance_total_fees_transfers_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix("astria".to_string()).unwrap();
        state_tx
            .put_native_asset(crate::test_utils::nria())
            .unwrap();
        let transfer_fees = FeeComponents::TransferFeeComponents(TransferFeeComponents {
            base_fee: 12,
            computed_cost_multiplier: 0,
        });
        state_tx
            .put_transfer_fees(transfer_fees)
            .wrap_err("failed to initiate transfer fee components")
            .unwrap();

        let sequence_fees = FeeComponents::SequenceFeeComponents(SequenceFeeComponents {
            base_fee: 0,
            computed_cost_multiplier: 1,
        });
        state_tx
            .put_sequence_fees(sequence_fees)
            .wrap_err("failed to initiate sequence action fee components")
            .unwrap();

        let ics20_withdrawal_fees =
            FeeComponents::Ics20WithdrawalFeeComponents(Ics20WithdrawalFeeComponents {
                base_fee: 1,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
            .wrap_err("failed to initiate ics20 withdrawal fee components")
            .unwrap();

        let init_bridge_account_fees =
            FeeComponents::InitBridgeAccountFeeComponents(InitBridgeAccountFeeComponents {
                base_fee: 12,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_init_bridge_account_fees(init_bridge_account_fees)
            .wrap_err("failed to initiate init bridge account fee components")
            .unwrap();

        let bridge_lock_fees = FeeComponents::BridgeLockFeeComponents(BridgeLockFeeComponents {
            base_fee: 0,
            computed_cost_multiplier: 1,
        });
        state_tx
            .put_bridge_lock_fees(bridge_lock_fees)
            .wrap_err("failed to initiate bridge lock fee components")
            .unwrap();

        let bridge_unlock_fees =
            FeeComponents::BridgeUnlockFeeComponents(BridgeUnlockFeeComponents {
                base_fee: 0,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_bridge_unlock_fees(bridge_unlock_fees)
            .wrap_err("failed to initiate bridge unlock fee components")
            .unwrap();

        let bridge_sudo_change_fees =
            FeeComponents::BridgeSudoChangeFeeComponents(BridgeSudoChangeFeeComponents {
                base_fee: 24,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
            .wrap_err("failed to initiate bridge sudo change fee components")
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = TransferAction::fee_components(&state_tx)
            .await
            .unwrap()
            .unwrap()
            .base_fee;
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &crate::test_utils::nria(),
                transfer_fee
                    + crate::fees::calculate_sequence_action_fee_from_state(&data, &state_tx)
                        .await
                        .unwrap(),
            )
            .await
            .unwrap();
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &other_asset,
                amount,
            )
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(TransferAction {
                asset: other_asset.clone(),
                amount,
                fee_asset: crate::test_utils::nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: crate::test_utils::nria().into(),
            }),
        ];

        let tx = UnsignedTransaction::builder()
            .actions(actions)
            .chain_id("test-chain-id")
            .try_build()
            .unwrap();

        let signed_tx = tx.into_signed(&alice);
        check_balance_for_total_fees_and_transfers(&signed_tx, &state_tx)
            .await
            .expect("sufficient balance for all actions");
    }

    #[tokio::test]
    #[expect(clippy::too_many_lines, reason = "it's a test")]
    async fn check_balance_total_fees_and_transfers_insufficient_other_asset_balance() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state_tx
            .put_native_asset(crate::test_utils::nria())
            .unwrap();
        let transfer_fees = FeeComponents::TransferFeeComponents(TransferFeeComponents {
            base_fee: 12,
            computed_cost_multiplier: 0,
        });
        state_tx
            .put_transfer_fees(transfer_fees)
            .wrap_err("failed to initiate transfer fee components")
            .unwrap();

        let sequence_fees = FeeComponents::SequenceFeeComponents(SequenceFeeComponents {
            base_fee: 0,
            computed_cost_multiplier: 1,
        });
        state_tx
            .put_sequence_fees(sequence_fees)
            .wrap_err("failed to initiate sequence action fee components")
            .unwrap();

        let ics20_withdrawal_fees =
            FeeComponents::Ics20WithdrawalFeeComponents(Ics20WithdrawalFeeComponents {
                base_fee: 1,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
            .wrap_err("failed to initiate ics20 withdrawal fee components")
            .unwrap();

        let init_bridge_account_fees =
            FeeComponents::InitBridgeAccountFeeComponents(InitBridgeAccountFeeComponents {
                base_fee: 12,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_init_bridge_account_fees(init_bridge_account_fees)
            .wrap_err("failed to initiate init bridge account fee components")
            .unwrap();

        let bridge_lock_fees = FeeComponents::BridgeLockFeeComponents(BridgeLockFeeComponents {
            base_fee: 0,
            computed_cost_multiplier: 1,
        });
        state_tx
            .put_bridge_lock_fees(bridge_lock_fees)
            .wrap_err("failed to initiate bridge lock fee components")
            .unwrap();

        let bridge_unlock_fees =
            FeeComponents::BridgeUnlockFeeComponents(BridgeUnlockFeeComponents {
                base_fee: 0,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_bridge_unlock_fees(bridge_unlock_fees)
            .wrap_err("failed to initiate bridge unlock fee components")
            .unwrap();

        let bridge_sudo_change_fees =
            FeeComponents::BridgeSudoChangeFeeComponents(BridgeSudoChangeFeeComponents {
                base_fee: 24,
                computed_cost_multiplier: 0,
            });
        state_tx
            .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
            .wrap_err("failed to initiate bridge sudo change fee components")
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = TransferAction::fee_components(&state_tx)
            .await
            .unwrap()
            .unwrap()
            .base_fee;
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &crate::test_utils::nria(),
                transfer_fee
                    + crate::fees::calculate_sequence_action_fee_from_state(&data, &state_tx)
                        .await
                        .unwrap(),
            )
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(TransferAction {
                asset: other_asset.clone(),
                amount,
                fee_asset: crate::test_utils::nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: crate::test_utils::nria().into(),
            }),
        ];

        let tx = UnsignedTransaction::builder()
            .actions(actions)
            .chain_id("test-chain-id")
            .try_build()
            .unwrap();

        let signed_tx = tx.into_signed(&alice);
        let err = check_balance_for_total_fees_and_transfers(&signed_tx, &state_tx)
            .await
            .err()
            .unwrap();
        assert!(
            err.root_cause()
                .to_string()
                .contains(&other_asset.to_ibc_prefixed().to_string())
        );
    }
}
