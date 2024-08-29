use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1alpha1::{
        action::Action,
        SignedTransaction,
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
    bridge::StateReadExt as _,
    state_ext::StateReadExt as _,
    transaction::fees::{
        get_and_report_tx_fees,
        FeeInfo,
    },
};

pub(crate) type PaymentMap = HashMap<([u8; 20], [u8; 20], asset::Denom), FeeInfo>;

#[instrument(skip_all)]
pub(crate) async fn check_nonce_mempool<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> Result<()> {
    let signer_address = state
        .try_base_prefixed(&tx.verification_key().address_bytes())
        .await
        .wrap_err(
            "failed constructing the signer address from signed transaction verification and \
             prefix provided by app state",
        )?;
    let curr_nonce = state
        .get_account_nonce(signer_address)
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
pub(crate) async fn check_balance_mempool<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
) -> Result<()> {
    check_balance_and_get_fees(tx, state, false)
        .await
        .wrap_err("failed to check balance for total fees and transfers")
        .map_err(|e| astria_eyre::eyre::eyre!(format!("{:?}", e)))?;
    Ok(())
}

// Checks that the account has enough balance to cover the total fees and transferred values
// for all actions in the transaction.
#[instrument(skip_all)]
pub(crate) async fn check_balance_and_get_fees<S: StateRead>(
    tx: &SignedTransaction,
    state: &S,
    return_payment_map: bool,
) -> Result<Option<PaymentMap>> {
    let (mut cost_by_asset, payment_map_option) =
        get_and_report_tx_fees(tx.unsigned_transaction(), state, return_payment_map)
            .await
            .wrap_err("failed to get fees for transaction")?;

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
                    .wrap_err("failed to get bridge account asset id")?;
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
            .wrap_err("failed to get account balance")?;
        ensure!(
            balance >= total_fee,
            "insufficient funds for asset {}",
            asset
        );
    }

    Ok(payment_map_option)
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            ADDRESS_LEN,
        },
        protocol::transaction::v1alpha1::{
            action::{
                SequenceAction,
                TransferAction,
            },
            TransactionParams,
            UnsignedTransaction,
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
        app::test_utils::{
            calculate_fee_from_state,
            get_alice_signing_key,
        },
        assets::StateWriteExt as _,
        bridge::StateWriteExt as _,
        ibc::StateWriteExt as _,
        sequence::StateWriteExt as _,
    };

    #[tokio::test]
    async fn check_balance_mempool_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix("astria").unwrap();
        state_tx.put_native_asset(&crate::test_utils::nria());
        state_tx.put_transfer_base_fee(12).unwrap();
        state_tx.put_sequence_action_base_fee(0);
        state_tx.put_sequence_action_byte_cost_multiplier(1);
        state_tx.put_ics20_withdrawal_base_fee(1).unwrap();
        state_tx.put_init_bridge_account_base_fee(12);
        state_tx.put_bridge_lock_byte_cost_multiplier(1);
        state_tx.put_bridge_sudo_change_base_fee(24);

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx.get_transfer_base_fee().await.unwrap();
        state_tx
            .increase_balance(
                state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                crate::test_utils::nria(),
                transfer_fee + calculate_fee_from_state(&data, &state_tx).await.unwrap(),
            )
            .await
            .unwrap();
        state_tx
            .increase_balance(
                state_tx
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

        let params = TransactionParams::builder()
            .nonce(0)
            .chain_id("test-chain-id")
            .build();
        let tx = UnsignedTransaction {
            actions,
            params,
        };

        let signed_tx = tx.into_signed(&alice);
        check_balance_mempool(&signed_tx, &state_tx)
            .await
            .expect("sufficient balance for all actions");
    }

    #[tokio::test]
    async fn check_balance_mempool_insufficient_other_asset_balance() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_base_prefix("nria").unwrap();
        state_tx.put_native_asset(&crate::test_utils::nria());
        state_tx.put_transfer_base_fee(12).unwrap();
        state_tx.put_sequence_action_base_fee(0);
        state_tx.put_sequence_action_byte_cost_multiplier(1);
        state_tx.put_ics20_withdrawal_base_fee(1).unwrap();
        state_tx.put_init_bridge_account_base_fee(12);
        state_tx.put_bridge_lock_byte_cost_multiplier(1);
        state_tx.put_bridge_sudo_change_base_fee(24);

        let other_asset = "other".parse::<Denom>().unwrap();

        let alice = get_alice_signing_key();
        let amount = 100;
        let data = Bytes::from_static(&[0; 32]);
        let transfer_fee = state_tx.get_transfer_base_fee().await.unwrap();
        state_tx
            .increase_balance(
                state_tx
                    .try_base_prefixed(&alice.address_bytes())
                    .await
                    .unwrap(),
                crate::test_utils::nria(),
                transfer_fee + calculate_fee_from_state(&data, &state_tx).await.unwrap(),
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

        let params = TransactionParams::builder()
            .nonce(0)
            .chain_id("test-chain-id")
            .build();
        let tx = UnsignedTransaction {
            actions,
            params,
        };

        let signed_tx = tx.into_signed(&alice);
        let err = check_balance_mempool(&signed_tx, &state_tx)
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
