use std::collections::HashMap;

use anyhow::{
    ensure,
    Context as _,
};
use astria_core::{
    primitive::v1::{
        asset,
        Address,
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

use crate::{
    accounts::state_ext::StateReadExt,
    bridge::state_ext::StateReadExt as _,
    ibc::state_ext::StateReadExt as _,
    state_ext::StateReadExt as _,
};

pub(crate) async fn check_nonce_mempool<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let signer_address = *tx.verification_key().address();
    let curr_nonce = state
        .get_account_nonce(signer_address)
        .await
        .context("failed to get account nonce")?;
    ensure!(
        tx.unsigned_transaction().params.nonce >= curr_nonce,
        "nonce already used by account"
    );
    Ok(())
}

pub(crate) async fn check_chain_id_mempool<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let chain_id = state
        .get_chain_id()
        .await
        .context("failed to get chain id")?;
    ensure!(
        tx.unsigned_transaction().params.chain_id == chain_id.as_str(),
        "chain id mismatch"
    );
    Ok(())
}

pub(crate) async fn check_balance_mempool<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let signer_address = *tx.verification_key().address();
    check_balance_for_total_fees(tx.unsigned_transaction(), signer_address, state).await?;
    Ok(())
}

// Checks that the account has enough balance to cover the total fees and transferred values
// for all actions in the transaction.
pub(crate) async fn check_balance_for_total_fees<S: StateReadExt + 'static>(
    tx: &UnsignedTransaction,
    from: Address,
    state: &S,
) -> anyhow::Result<()> {
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
            Action::Transfer(act) => transfer_update_fees(
                act.asset_id,
                act.fee_asset_id,
                act.amount,
                &mut fees_by_asset,
                transfer_fee,
            ),
            Action::Sequence(act) => {
                sequence_update_fees(state, act.fee_asset_id, &mut fees_by_asset, &act.data)
                    .await?;
            }
            Action::Ics20Withdrawal(act) => ics20_withdrawal_updates_fees(
                act.denom().id(),
                *act.fee_asset_id(),
                act.amount(),
                &mut fees_by_asset,
                ics20_withdrawal_fee,
            ),
            Action::InitBridgeAccount(act) => {
                fees_by_asset
                    .entry(act.fee_asset_id)
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
                bridge_unlock_update_fees(
                    state,
                    act.bridge_address.unwrap_or(from),
                    act.amount,
                    act.fee_asset_id,
                    &mut fees_by_asset,
                    transfer_fee,
                )
                .await?;
            }
            Action::BridgeSudoChange(act) => {
                fees_by_asset
                    .entry(act.fee_asset_id)
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
    for (asset, total_fee) in fees_by_asset {
        let balance = state
            .get_account_balance(from, asset)
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
    asset_id: asset::Id,
    fee_asset_id: asset::Id,
    amount: u128,
    fees_by_asset: &mut HashMap<asset::Id, u128>,
    transfer_fee: u128,
) {
    fees_by_asset
        .entry(asset_id)
        .and_modify(|amt: &mut u128| *amt = amt.saturating_add(amount))
        .or_insert(amount);
    fees_by_asset
        .entry(fee_asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(transfer_fee))
        .or_insert(transfer_fee);
}

async fn sequence_update_fees<S: StateReadExt>(
    state: &S,
    fee_asset_id: asset::Id,
    fees_by_asset: &mut HashMap<asset::Id, u128>,
    data: &[u8],
) -> anyhow::Result<()> {
    let fee = crate::sequence::calculate_fee_from_state(data, state)
        .await
        .context("fee for sequence action overflowed; data too large")?;
    fees_by_asset
        .entry(fee_asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(fee))
        .or_insert(fee);
    Ok(())
}

fn ics20_withdrawal_updates_fees(
    asset_id: asset::Id,
    fee_asset_id: asset::Id,
    amount: u128,
    fees_by_asset: &mut HashMap<asset::Id, u128>,
    ics20_withdrawal_fee: u128,
) {
    fees_by_asset
        .entry(asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(amount))
        .or_insert(amount);
    fees_by_asset
        .entry(fee_asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(ics20_withdrawal_fee))
        .or_insert(ics20_withdrawal_fee);
}

fn bridge_lock_update_fees(
    act: &BridgeLockAction,
    fees_by_asset: &mut HashMap<asset::Id, u128>,
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
            act.asset_id,
            act.destination_chain_address.clone(),
        ))
        .saturating_mul(bridge_lock_byte_cost_multiplier),
    );

    fees_by_asset
        .entry(act.asset_id)
        .and_modify(|amt: &mut u128| *amt = amt.saturating_add(act.amount))
        .or_insert(act.amount);
    fees_by_asset
        .entry(act.asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(expected_deposit_fee))
        .or_insert(expected_deposit_fee);
}

async fn bridge_unlock_update_fees<S: StateReadExt>(
    state: &S,
    bridge_address: Address,
    amount: u128,
    fee_asset_id: asset::Id,
    fees_by_asset: &mut HashMap<asset::Id, u128>,
    transfer_fee: u128,
) -> anyhow::Result<()> {
    let asset_id = state
        .get_bridge_account_asset_id(&bridge_address)
        .await
        .context("must be a bridge account for BridgeUnlock action")?;
    fees_by_asset
        .entry(asset_id)
        .and_modify(|amt: &mut u128| *amt = amt.saturating_add(amount))
        .or_insert(amount);
    fees_by_asset
        .entry(fee_asset_id)
        .and_modify(|amt| *amt = amt.saturating_add(transfer_fee))
        .or_insert(transfer_fee);
    Ok(())
}

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset::{
                Denom,
                DEFAULT_NATIVE_ASSET_DENOM,
            },
            RollupId,
            ADDRESS_LEN,
        },
        protocol::transaction::v1alpha1::{
            action::{
                SequenceAction,
                TransferAction,
            },
            TransactionParams,
        },
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        app::test_utils::*,
        bridge::state_ext::StateWriteExt,
        ibc::state_ext::StateWriteExt as _,
        sequence::state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn check_balance_mempool_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_transfer_base_fee(12).unwrap();
        state_tx.put_sequence_action_base_fee(0);
        state_tx.put_sequence_action_byte_cost_multiplier(1);
        state_tx.put_ics20_withdrawal_base_fee(1).unwrap();
        state_tx.put_init_bridge_account_base_fee(12);
        state_tx.put_bridge_lock_byte_cost_multiplier(1);
        state_tx.put_bridge_sudo_change_base_fee(24);

        crate::asset::initialize_native_asset(DEFAULT_NATIVE_ASSET_DENOM);
        let native_asset = crate::asset::get_native_asset().id();
        let other_asset = Denom::from_base_denom("other").id();

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let amount = 100;
        let data = [0; 32].to_vec();
        let transfer_fee = state_tx.get_transfer_base_fee().await.unwrap();
        state_tx
            .increase_balance(
                alice_address,
                native_asset,
                transfer_fee
                    + crate::sequence::calculate_fee_from_state(&data, &state_tx)
                        .await
                        .unwrap(),
            )
            .await
            .unwrap();
        state_tx
            .increase_balance(alice_address, other_asset, amount)
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(TransferAction {
                asset_id: other_asset,
                amount,
                fee_asset_id: native_asset,
                to: [0; ADDRESS_LEN].into(),
            }),
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset_id: native_asset,
            }),
        ];

        let params = TransactionParams {
            nonce: 0,
            chain_id: "test-chain-id".to_string(),
        };
        let tx = UnsignedTransaction {
            actions,
            params,
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        check_balance_mempool(&signed_tx, &state_tx)
            .await
            .expect("sufficient balance for all actions");
    }

    #[tokio::test]
    async fn check_balance_mempool_insufficient_other_asset_balance() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_transfer_base_fee(12).unwrap();
        state_tx.put_sequence_action_base_fee(0);
        state_tx.put_sequence_action_byte_cost_multiplier(1);
        state_tx.put_ics20_withdrawal_base_fee(1).unwrap();
        state_tx.put_init_bridge_account_base_fee(12);
        state_tx.put_bridge_lock_byte_cost_multiplier(1);
        state_tx.put_bridge_sudo_change_base_fee(24);

        crate::asset::initialize_native_asset(DEFAULT_NATIVE_ASSET_DENOM);
        let native_asset = crate::asset::get_native_asset().id();
        let other_asset = Denom::from_base_denom("other").id();

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let amount = 100;
        let data = [0; 32].to_vec();
        let transfer_fee = state_tx.get_transfer_base_fee().await.unwrap();
        state_tx
            .increase_balance(
                alice_address,
                native_asset,
                transfer_fee
                    + crate::sequence::calculate_fee_from_state(&data, &state_tx)
                        .await
                        .unwrap(),
            )
            .await
            .unwrap();

        let actions = vec![
            Action::Transfer(TransferAction {
                asset_id: other_asset,
                amount,
                fee_asset_id: native_asset,
                to: [0; ADDRESS_LEN].into(),
            }),
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data,
                fee_asset_id: native_asset,
            }),
        ];

        let params = TransactionParams {
            nonce: 0,
            chain_id: "test-chain-id".to_string(),
        };
        let tx = UnsignedTransaction {
            actions,
            params,
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let err = check_balance_mempool(&signed_tx, &state_tx)
            .await
            .expect_err("insufficient funds for `other` asset");
        assert!(err.to_string().contains(&other_asset.to_string()));
    }
}
