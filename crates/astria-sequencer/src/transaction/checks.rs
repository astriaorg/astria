use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset::{
        self,
    },
    protocol::transaction::v1::{
        action::Action,
        Transaction,
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
    fees::query::get_fees_for_transaction,
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
            | Action::RollupDataSubmission(_)
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

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            ADDRESS_LEN,
        },
        protocol::{
            fees::v1::FeeComponents,
            transaction::v1::{
                action::{
                    BridgeLock,
                    BridgeSudoChange,
                    BridgeUnlock,
                    Ics20Withdrawal,
                    InitBridgeAccount,
                    RollupDataSubmission,
                    Transfer,
                },
                TransactionBody,
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
        benchmark_and_test_utils::{
            nria,
            ASTRIA_PREFIX,
        },
        fees::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        test_utils::calculate_rollup_data_submission_fee_from_state,
    };

    #[tokio::test]
    async fn check_balance_total_fees_transfers_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix("astria".to_string()).unwrap();
        state_tx.put_native_asset(nria()).unwrap();
        state_tx
            .put_fees(FeeComponents::<Transfer>::new(12, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<RollupDataSubmission>::new(0, 1))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<Ics20Withdrawal>::new(1, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<InitBridgeAccount>::new(12, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeLock>::new(0, 1))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeUnlock>::new(0, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeSudoChange>::new(24, 0))
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx
            .get_fees::<Transfer>()
            .await
            .expect("should not error fetching transfer fees")
            .expect("transfer fees should be stored")
            .base();
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &nria(),
                transfer_fee
                    + calculate_rollup_data_submission_fee_from_state(&data, &state_tx).await,
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
                fee_asset: nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::RollupDataSubmission(RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: nria().into(),
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
    async fn check_balance_total_fees_and_transfers_insufficient_other_asset_balance() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state_tx.put_native_asset(nria()).unwrap();
        state_tx
            .put_fees(FeeComponents::<Transfer>::new(12, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<RollupDataSubmission>::new(0, 1))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<Ics20Withdrawal>::new(1, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<InitBridgeAccount>::new(12, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeLock>::new(0, 1))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeUnlock>::new(0, 0))
            .unwrap();
        state_tx
            .put_fees(FeeComponents::<BridgeSudoChange>::new(24, 0))
            .unwrap();

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx
            .get_fees::<Transfer>()
            .await
            .expect("should not error fetching transfer fees")
            .expect("transfer fees should be stored")
            .base();
        state_tx
            .increase_balance(
                &state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                &nria(),
                transfer_fee
                    + calculate_rollup_data_submission_fee_from_state(&data, &state_tx).await,
            )
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(Transfer {
                asset: other_asset.clone(),
                amount,
                fee_asset: nria().into(),
                to: state_tx.try_base_prefixed(&[0; ADDRESS_LEN]).await.unwrap(),
            }),
            Action::RollupDataSubmission(RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset: nria().into(),
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
