pub(crate) mod action_handler;

use std::fmt;

pub(crate) use action_handler::ActionHandler;
#[cfg(not(feature = "mint"))]
use anyhow::bail;
use anyhow::{
    ensure,
    Context as _,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::{
        action::Action,
        SignedTransaction,
        UnsignedTransaction,
    },
};
use tracing::instrument;

use crate::{
    accounts::{
        state_ext::{
            StateReadExt,
            StateWriteExt,
        },
    },
    bridge::init_bridge_account_action::INIT_BRIDGE_ACCOUNT_FEE,
    ibc::{
        host_interface::AstriaHost,
        ics20_withdrawal::ICS20_WITHDRAWAL_FEE,
        state_ext::StateReadExt as _,
    },
    state_ext::StateReadExt as _,
};

pub(crate) async fn check_nonce_mempool<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let signer_address = Address::from_verification_key(tx.verification_key());
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
    use std::collections::HashMap;
    
    let transfer_fee = state.get_transfer_base_fee().await.context("failed to get transfer base fee")?;

    let signer_address = Address::from_verification_key(tx.verification_key());
    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                fees_by_asset
                    .entry(act.asset_id)
                    .and_modify(|amt| *amt += act.amount)
                    .or_insert(act.amount);
                fees_by_asset
                    .entry(act.fee_asset_id)
                    .and_modify(|amt| *amt += transfer_fee)
                    .or_insert(transfer_fee);
            }
            Action::Sequence(act) => {
                let fee = crate::sequence::calculate_fee(&act.data)
                    .context("fee for sequence action overflowed; data too large")?;
                fees_by_asset
                    .entry(act.fee_asset_id)
                    .and_modify(|amt| *amt += fee)
                    .or_insert(fee);
            }
            Action::Ics20Withdrawal(act) => {
                fees_by_asset
                    .entry(act.denom().id())
                    .and_modify(|amt| *amt += act.amount())
                    .or_insert(act.amount());
                fees_by_asset
                    .entry(*act.fee_asset_id())
                    .and_modify(|amt| *amt += ICS20_WITHDRAWAL_FEE)
                    .or_insert(ICS20_WITHDRAWAL_FEE);
            }
            Action::InitBridgeAccount(act) => {
                fees_by_asset
                    .entry(act.fee_asset_id)
                    .and_modify(|amt| *amt += INIT_BRIDGE_ACCOUNT_FEE)
                    .or_insert(INIT_BRIDGE_ACCOUNT_FEE);
            }
            Action::BridgeLock(act) => {
                fees_by_asset
                    .entry(act.asset_id)
                    .and_modify(|amt| *amt += act.amount)
                    .or_insert(act.amount);
                fees_by_asset
                    .entry(act.fee_asset_id)
                    .and_modify(|amt| *amt += transfer_fee)
                    .or_insert(transfer_fee);
            }
            Action::ValidatorUpdate(_)
            | Action::SudoAddressChange(_)
            | Action::Ibc(_)
            | Action::IbcRelayerChange(_)
            | Action::FeeAssetChange(_)
            | Action::Mint(_) => {
                continue;
            }
        }
    }
    for (asset, total_fee) in fees_by_asset {
        let balance = state
            .get_account_balance(signer_address, asset)
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

pub(crate) async fn check_stateless(tx: &SignedTransaction) -> anyhow::Result<()> {
    tx.unsigned_transaction()
        .check_stateless()
        .await
        .context("stateless check failed")
}

pub(crate) async fn check_stateful<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let signer_address = Address::from_verification_key(tx.verification_key());
    tx.unsigned_transaction()
        .check_stateful(state, signer_address)
        .await
}

pub(crate) async fn execute<S: StateWriteExt>(
    tx: &SignedTransaction,
    state: &mut S,
) -> anyhow::Result<()> {
    let signer_address = Address::from_verification_key(tx.verification_key());
    tx.unsigned_transaction()
        .execute(state, signer_address)
        .await
}

#[derive(Debug)]
pub(crate) struct InvalidChainId(pub(crate) String);

impl fmt::Display for InvalidChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provided chain id {} does not match expected chain id",
            self.0,
        )
    }
}

impl std::error::Error for InvalidChainId {}

#[derive(Debug)]
pub(crate) struct InvalidNonce(pub(crate) u32);

impl fmt::Display for InvalidNonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provided nonce {} does not match expected next nonce",
            self.0,
        )
    }
}

impl std::error::Error for InvalidNonce {}

#[async_trait::async_trait]
impl ActionHandler for UnsignedTransaction {
    async fn check_stateless(&self) -> anyhow::Result<()> {
        ensure!(!self.actions.is_empty(), "must have at least one action");

        for action in &self.actions {
            match action {
                Action::Transfer(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for TransferAction")?,
                Action::Sequence(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for SequenceAction")?,
                Action::ValidatorUpdate(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for ValidatorUpdateAction")?,
                Action::SudoAddressChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for SudoAddressChangeAction")?,
                Action::Ibc(act) => {
                    let action = act
                        .clone()
                        .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                    action
                        .check_stateless(())
                        .await
                        .context("stateless check failed for IbcAction")?;
                }
                Action::Ics20Withdrawal(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for Ics20WithdrawalAction")?,
                Action::IbcRelayerChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for IbcRelayerChangeAction")?,
                Action::FeeAssetChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for FeeAssetChangeAction")?,
                Action::InitBridgeAccount(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for InitBridgeAccountAction")?,
                Action::BridgeLock(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for BridgeLockAction")?,
                #[cfg(feature = "mint")]
                Action::Mint(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for MintAction")?,
                #[cfg(not(feature = "mint"))]
                _ => bail!("unsupported action type: {:?}", action),
            }
        }
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> anyhow::Result<()> {
        // Transactions must match the chain id of the node.
        let chain_id = state.get_chain_id().await?;
        ensure!(
            self.params.chain_id == chain_id.as_str(),
            InvalidChainId(self.params.chain_id.clone())
        );

        // Nonce should be equal to the number of executed transactions before this tx.
        // First tx has nonce 0.
        let curr_nonce = state.get_account_nonce(from).await?;
        ensure!(
            curr_nonce == self.params.nonce,
            InvalidNonce(self.params.nonce)
        );

        for action in &self.actions {
            match action {
                Action::Transfer(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for TransferAction")?,
                Action::Sequence(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for SequenceAction")?,
                Action::ValidatorUpdate(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for ValidatorUpdateAction")?,
                Action::SudoAddressChange(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for SudoAddressChangeAction")?,
                Action::Ibc(_) => {
                    ensure!(
                        state
                            .is_ibc_relayer(&from)
                            .await
                            .context("failed to check if address is IBC relayer")?,
                        "only IBC sudo address can execute IBC actions"
                    );
                }
                Action::Ics20Withdrawal(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for Ics20WithdrawalAction")?,
                Action::IbcRelayerChange(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for IbcRelayerChangeAction")?,
                Action::FeeAssetChange(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for FeeAssetChangeAction")?,
                Action::InitBridgeAccount(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for InitBridgeAccountAction")?,
                Action::BridgeLock(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for BridgeLockAction")?,
                #[cfg(feature = "mint")]
                Action::Mint(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for MintAction")?,
                #[cfg(not(feature = "mint"))]
                _ => bail!("unsupported action type: {:?}", action),
            }
        }

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            nonce = self.params.nonce,
            from = from.to_string(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> anyhow::Result<()> {
        let from_nonce = state
            .get_account_nonce(from)
            .await
            .context("failed getting `from` nonce")?;
        let next_nonce = from_nonce
            .checked_add(1)
            .context("overflow occured incrementing stored nonce")?;
        state
            .put_account_nonce(from, next_nonce)
            .context("failed updating `from` nonce")?;

        for action in &self.actions {
            match action {
                Action::Transfer(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for TransferAction")?;
                }
                Action::Sequence(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for SequenceAction")?;
                }
                Action::ValidatorUpdate(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for ValidatorUpdateAction")?;
                }
                Action::SudoAddressChange(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for SudoAddressChangeAction")?;
                }
                Action::Ibc(act) => {
                    let action = act
                        .clone()
                        .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                    action
                        .execute(&mut *state)
                        .await
                        .context("execution failed for IbcAction")?;
                }
                Action::Ics20Withdrawal(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for Ics20WithdrawalAction")?;
                }
                Action::IbcRelayerChange(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for IbcRelayerChangeAction")?;
                }
                Action::FeeAssetChange(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for FeeAssetChangeAction")?;
                }
                Action::InitBridgeAccount(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for InitBridgeAccountAction")?;
                }
                Action::BridgeLock(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for BridgeLockAction")?;
                }
                #[cfg(feature = "mint")]
                Action::Mint(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for MintAction")?;
                }
                #[cfg(not(feature = "mint"))]
                _ => bail!("unsupported action type: {:?}", action),
            }
        }

        Ok(())
    }
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
    use crate::app::test_utils::*;

    #[tokio::test]
    async fn check_balance_mempool_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot);

        state_tx.put_transfer_base_fee(crate::accounts::component::DEFAULT_TRANSFER_BASE_FEE).unwrap();
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
                transfer_fee + crate::sequence::calculate_fee(&data).unwrap(),
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
        
        state_tx.put_transfer_base_fee(crate::accounts::component::DEFAULT_TRANSFER_BASE_FEE).unwrap();
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
                transfer_fee + crate::sequence::calculate_fee(&data).unwrap(),
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
