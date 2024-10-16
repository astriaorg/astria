use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset,
    protocol::{
        fees::v1alpha1::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            BridgeUnlockFeeComponents,
            Ics20WithdrawalFeeComponents,
            InitBridgeAccountFeeComponents,
            SequenceFeeComponents,
            TransferFeeComponents,
        },
        transaction::v1alpha1::{
            action::{
                Action,
                BridgeLock,
                BridgeSudoChange,
                BridgeUnlock,
                Ics20Withdrawal,
                InitBridgeAccount,
                Sequence,
                Transfer,
            },
            Transaction,
            TransactionBody,
        },
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
    app::StateReadExt as _,
    bridge::StateReadExt as _,
    fees::{
        FeeHandler,
        StateReadExt as _,
    },
};

#[instrument(skip_all)]
pub(crate) async fn check_chain_id_mempool<S: StateRead>(
    tx: &Transaction,
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
    tx: &TransactionBody,
    state: &S,
) -> Result<HashMap<asset::IbcPrefixed, u128>> {
    let transfer_fees = state
        .get_transfer_fees()
        .await
        .wrap_err("failed to get transfer fees")?;
    let sequence_fees = state
        .get_sequence_fees()
        .await
        .wrap_err("failed to get sequence fees")?;
    let ics20_withdrawal_fees = state
        .get_ics20_withdrawal_fees()
        .await
        .wrap_err("failed to get ics20 withdrawal fees")?;
    let init_bridge_account_fees = state
        .get_init_bridge_account_fees()
        .await
        .wrap_err("failed to get init bridge account fees")?;
    let bridge_lock_fees = state
        .get_bridge_lock_fees()
        .await
        .wrap_err("failed to get bridge lock fees")?;
    let bridge_unlock_fees = state
        .get_bridge_unlock_fees()
        .await
        .wrap_err("failed to get bridge unlock fees")?;
    let bridge_sudo_change_fees = state
        .get_bridge_sudo_change_fees()
        .await
        .wrap_err("failed to get bridge sudo change fees")?;

    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                transfer_update_fees(act, &mut fees_by_asset, &transfer_fees);
            }
            Action::Sequence(act) => {
                sequence_update_fees(act, &mut fees_by_asset, &sequence_fees);
            }
            Action::Ics20Withdrawal(act) => {
                ics20_withdrawal_updates_fees(act, &mut fees_by_asset, &ics20_withdrawal_fees);
            }
            Action::InitBridgeAccount(act) => {
                init_bridge_account_update_fees(act, &mut fees_by_asset, &init_bridge_account_fees);
            }
            Action::BridgeLock(act) => {
                bridge_lock_update_fees(act, &mut fees_by_asset, &bridge_lock_fees);
            }
            Action::BridgeUnlock(act) => {
                bridge_unlock_update_fees(act, &mut fees_by_asset, &bridge_unlock_fees);
            }
            Action::BridgeSudoChange(act) => {
                bridge_sudo_change_update_fees(act, &mut fees_by_asset, &bridge_sudo_change_fees);
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
    tx: &Transaction,
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
    tx: &Transaction,
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
    act: &Transfer,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    transfer_fees: &TransferFeeComponents,
) {
    let total_fees = calculate_total_fees(
        transfer_fees.base,
        transfer_fees.multiplier,
        act.variable_component(),
    );
    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn sequence_update_fees(
    act: &Sequence,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    sequence_fees: &SequenceFeeComponents,
) {
    let total_fees = calculate_total_fees(
        sequence_fees.base,
        sequence_fees.multiplier,
        act.variable_component(),
    );
    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn ics20_withdrawal_updates_fees(
    act: &Ics20Withdrawal,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    ics20_withdrawal_fees: &Ics20WithdrawalFeeComponents,
) {
    let total_fees = calculate_total_fees(
        ics20_withdrawal_fees.base,
        ics20_withdrawal_fees.multiplier,
        act.variable_component(),
    );
    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_lock_update_fees(
    act: &BridgeLock,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_lock_fees: &BridgeLockFeeComponents,
) {
    let total_fees = calculate_total_fees(
        bridge_lock_fees.base,
        bridge_lock_fees.multiplier,
        act.variable_component(),
    );

    fees_by_asset
        .entry(act.asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn init_bridge_account_update_fees(
    act: &InitBridgeAccount,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    init_bridge_account_fees: &InitBridgeAccountFeeComponents,
) {
    let total_fees = calculate_total_fees(
        init_bridge_account_fees.base,
        init_bridge_account_fees.multiplier,
        act.variable_component(),
    );

    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_unlock_update_fees(
    act: &BridgeUnlock,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_lock_fees: &BridgeUnlockFeeComponents,
) {
    let total_fees = calculate_total_fees(
        bridge_lock_fees.base,
        bridge_lock_fees.multiplier,
        act.variable_component(),
    );

    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn bridge_sudo_change_update_fees(
    act: &BridgeSudoChange,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    bridge_sudo_change_fees: &BridgeSudoChangeFeeComponents,
) {
    let total_fees = calculate_total_fees(
        bridge_sudo_change_fees.base,
        bridge_sudo_change_fees.multiplier,
        act.variable_component(),
    );

    fees_by_asset
        .entry(act.fee_asset.to_ibc_prefixed())
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn calculate_total_fees(base: u128, multiplier: u128, computed_cost_base: u128) -> u128 {
    base.saturating_add(computed_cost_base.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            ADDRESS_LEN,
        },
        protocol::{
            fees::v1alpha1::{
                BridgeLockFeeComponents,
                BridgeSudoChangeFeeComponents,
                BridgeUnlockFeeComponents,
                Ics20WithdrawalFeeComponents,
                InitBridgeAccountFeeComponents,
                SequenceFeeComponents,
                TransferFeeComponents,
            },
            transaction::v1alpha1::action::{
                Sequence,
                Transfer,
            },
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
        fees::StateWriteExt as _,
        test_utils::{
            calculate_sequence_action_fee_from_state,
            ASTRIA_PREFIX,
        },
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
        let transfer_fees = TransferFeeComponents {
            base: 12,
            multiplier: 0,
        };
        state_tx
            .put_transfer_fees(transfer_fees)
            .wrap_err("failed to initiate transfer fee components")
            .unwrap();

        let sequence_fees = SequenceFeeComponents {
            base: 0,
            multiplier: 1,
        };
        state_tx
            .put_sequence_fees(sequence_fees)
            .wrap_err("failed to initiate sequence action fee components")
            .unwrap();

        let ics20_withdrawal_fees = Ics20WithdrawalFeeComponents {
            base: 1,
            multiplier: 0,
        };
        state_tx
            .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
            .wrap_err("failed to initiate ics20 withdrawal fee components")
            .unwrap();

        let init_bridge_account_fees = InitBridgeAccountFeeComponents {
            base: 12,
            multiplier: 0,
        };
        state_tx
            .put_init_bridge_account_fees(init_bridge_account_fees)
            .wrap_err("failed to initiate init bridge account fee components")
            .unwrap();

        let bridge_lock_fees = BridgeLockFeeComponents {
            base: 0,
            multiplier: 1,
        };
        state_tx
            .put_bridge_lock_fees(bridge_lock_fees)
            .wrap_err("failed to initiate bridge lock fee components")
            .unwrap();

        let bridge_unlock_fees = BridgeUnlockFeeComponents {
            base: 0,
            multiplier: 0,
        };
        state_tx
            .put_bridge_unlock_fees(bridge_unlock_fees)
            .wrap_err("failed to initiate bridge unlock fee components")
            .unwrap();

        let bridge_sudo_change_fees = BridgeSudoChangeFeeComponents {
            base: 24,
            multiplier: 0,
        };
        state_tx
            .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
            .wrap_err("failed to initiate bridge sudo change fee components")
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx.get_transfer_fees().await.unwrap().base;
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &crate::test_utils::nria(),
                transfer_fee + calculate_sequence_action_fee_from_state(&data, &state_tx).await,
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
            Action::Transfer(Transfer {
                asset: other_asset.clone(),
                amount,
                fee_asset: crate::test_utils::nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::Sequence(Sequence {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: crate::test_utils::nria().into(),
            }),
        ];

        let tx = TransactionBody::builder()
            .actions(actions)
            .chain_id("test-chain-id")
            .try_build()
            .unwrap();

        let signed_tx = tx.sign(&alice);
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
        let transfer_fees = TransferFeeComponents {
            base: 12,
            multiplier: 0,
        };
        state_tx
            .put_transfer_fees(transfer_fees)
            .wrap_err("failed to initiate transfer fee components")
            .unwrap();

        let sequence_fees = SequenceFeeComponents {
            base: 0,
            multiplier: 1,
        };
        state_tx
            .put_sequence_fees(sequence_fees)
            .wrap_err("failed to initiate sequence action fee components")
            .unwrap();

        let ics20_withdrawal_fees = Ics20WithdrawalFeeComponents {
            base: 1,
            multiplier: 0,
        };
        state_tx
            .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
            .wrap_err("failed to initiate ics20 withdrawal fee components")
            .unwrap();

        let init_bridge_account_fees = InitBridgeAccountFeeComponents {
            base: 12,
            multiplier: 0,
        };
        state_tx
            .put_init_bridge_account_fees(init_bridge_account_fees)
            .wrap_err("failed to initiate init bridge account fee components")
            .unwrap();

        let bridge_lock_fees = BridgeLockFeeComponents {
            base: 0,
            multiplier: 1,
        };
        state_tx
            .put_bridge_lock_fees(bridge_lock_fees)
            .wrap_err("failed to initiate bridge lock fee components")
            .unwrap();

        let bridge_unlock_fees = BridgeUnlockFeeComponents {
            base: 0,
            multiplier: 0,
        };
        state_tx
            .put_bridge_unlock_fees(bridge_unlock_fees)
            .wrap_err("failed to initiate bridge unlock fee components")
            .unwrap();

        let bridge_sudo_change_fees = BridgeSudoChangeFeeComponents {
            base: 24,
            multiplier: 0,
        };
        state_tx
            .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
            .wrap_err("failed to initiate bridge sudo change fee components")
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx.get_transfer_fees().await.unwrap().base;
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &crate::test_utils::nria(),
                transfer_fee + calculate_sequence_action_fee_from_state(&data, &state_tx).await,
            )
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(Transfer {
                asset: other_asset.clone(),
                amount,
                fee_asset: crate::test_utils::nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::Sequence(Sequence {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: crate::test_utils::nria().into(),
            }),
        ];

        let tx = TransactionBody::builder()
            .actions(actions)
            .chain_id("test-chain-id")
            .try_build()
            .unwrap();

        let signed_tx = tx.sign(&alice);
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
