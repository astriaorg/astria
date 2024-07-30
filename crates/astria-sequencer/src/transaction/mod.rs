pub(crate) mod action_handler;
mod checks;
pub(crate) mod query;

use std::fmt;

pub(crate) use action_handler::ActionHandler;
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
pub(crate) use checks::{
    check_balance_for_total_fees_and_transfers,
    check_balance_mempool,
    check_chain_id_mempool,
    get_current_nonce_if_tx_nonce_valid,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    ibc::{
        host_interface::AstriaHost,
        state_ext::StateReadExt as _,
    },
    state_ext::StateReadExt as _,
};

#[instrument(skip_all)]
pub(crate) async fn check_stateless(tx: &SignedTransaction) -> anyhow::Result<()> {
    tx.unsigned_transaction()
        .check_stateless()
        .await
        .context("stateless check failed")
}

#[instrument(skip_all)]
pub(crate) async fn check_stateful<S: StateReadExt + 'static>(
    tx: &SignedTransaction,
    state: &S,
) -> anyhow::Result<()> {
    let signer_address = crate::address::base_prefixed(tx.verification_key().address_bytes());
    tx.unsigned_transaction()
        .check_stateful(state, signer_address)
        .await
}

pub(crate) async fn execute<S: StateWriteExt>(
    tx: &SignedTransaction,
    state: &mut S,
) -> anyhow::Result<()> {
    use crate::bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    let signer_address = crate::address::base_prefixed(tx.verification_key().address_bytes());

    if state
        .get_bridge_account_rollup_id(&signer_address)
        .await
        .context("failed to check account rollup id")?
        .is_some()
    {
        state.put_last_transaction_hash_for_bridge_account(
            &signer_address,
            &tx.sha256_of_proto_encoding(),
        );
    }

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
                Action::FeeChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for FeeChangeAction")?,
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
                Action::BridgeUnlock(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for BridgeLockAction")?,
                Action::BridgeSudoChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for BridgeSudoChangeAction")?,
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
            self.chain_id() == chain_id.as_str(),
            InvalidChainId(self.chain_id().to_string())
        );

        // Nonce should be equal to the number of executed transactions before this tx.
        // First tx has nonce 0.
        let curr_nonce = state.get_account_nonce(from).await?;
        ensure!(curr_nonce == self.nonce(), InvalidNonce(self.nonce()));

        // Should have enough balance to cover all actions.
        check_balance_for_total_fees_and_transfers(self, from, state)
            .await
            .context("failed to check balance for total fees and transfers")?;

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
                Action::FeeChange(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for FeeChangeAction")?,
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
                Action::BridgeUnlock(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for BridgeUnlockAction")?,
                Action::BridgeSudoChange(act) => act
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for BridgeSudoChangeAction")?,
            }
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> anyhow::Result<()> {
        let from_nonce = state
            .get_account_nonce(from)
            .await
            .context("failed getting `from` nonce")?;
        let next_nonce = from_nonce
            .checked_add(1)
            .context("overflow occurred incrementing stored nonce")?;
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
                Action::FeeChange(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for FeeChangeAction")?;
                }
                Action::Ibc(act) => {
                    let action = act
                        .clone()
                        .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                    action
                        .check_and_execute(&mut *state)
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
                Action::BridgeUnlock(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for BridgeUnlockAction")?;
                }
                Action::BridgeSudoChange(act) => {
                    act.execute(state, from)
                        .await
                        .context("execution failed for BridgeSudoChangeAction")?;
                }
            }
        }

        Ok(())
    }
}
