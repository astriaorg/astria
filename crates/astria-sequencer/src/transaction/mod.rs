mod checks;
pub(crate) mod query;
mod state_ext;

use std::fmt;

use anyhow::{
    ensure,
    Context as _,
};
use astria_core::protocol::transaction::v1alpha1::{
    action_groups::{
        ActionGroup,
        BundlableGeneralAction,
        BundlableSudoAction,
        GeneralAction,
        SudoAction,
    },
    SignedTransaction,
};
pub(crate) use checks::{
    check_balance_for_total_fees_and_transfers,
    check_chain_id_mempool,
    check_nonce_mempool,
    get_total_transaction_cost,
};
use cnidarium::StateWrite;
// Conditional to quiet warnings. This object is used throughout the codebase,
// but is never explicitly named - hence Rust warns about it being unused.
#[cfg(test)]
pub(crate) use state_ext::TransactionContext;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    app::ActionHandler,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::{
        host_interface::AstriaHost,
        StateReadExt as _,
    },
    state_ext::StateReadExt as _,
};

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
impl ActionHandler for SignedTransaction {
    async fn check_stateless(&self) -> anyhow::Result<()> {
        // ensure not emtpy
        match self.actions() {
            ActionGroup::BundlableGeneral(actions) => {
                ensure!(!actions.actions.is_empty(), "must have at least one action");
            }
            ActionGroup::BundlableSudo(actions) => {
                ensure!(!actions.actions.is_empty(), "must have at least one action");
            }
            ActionGroup::General(_) | ActionGroup::Sudo(_) => (),
        }

        match &self.actions() {
            ActionGroup::BundlableGeneral(actions) => {
                for action in &actions.actions {
                    match action {
                        BundlableGeneralAction::Transfer(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for TransferAction")?,
                        BundlableGeneralAction::Sequence(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for SequenceAction")?,
                        BundlableGeneralAction::ValidatorUpdate(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for ValidatorUpdateAction")?,
                        BundlableGeneralAction::Ibc(act) => {
                            let action = act
                                .clone()
                                .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                            action
                                .check_stateless(())
                                .await
                                .context("stateless check failed for IbcAction")?;
                        }
                        BundlableGeneralAction::Ics20Withdrawal(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for Ics20WithdrawalAction")?,
                        BundlableGeneralAction::BridgeLock(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for BridgeLockAction")?,
                        BundlableGeneralAction::BridgeUnlock(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for BridgeUnlockAction")?,
                    }
                }
            }
            ActionGroup::BundlableSudo(actions) => {
                for action in &actions.actions {
                    match action {
                        BundlableSudoAction::FeeChange(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for FeeChangeAction")?,
                        BundlableSudoAction::IbcRelayerChange(act) => {
                            act.check_stateless()
                                .await
                                .context("stateless check failed for IbcRelayerChangeAction")?;
                        }
                        BundlableSudoAction::FeeAssetChange(act) => act
                            .check_stateless()
                            .await
                            .context("stateless check failed for FeeAssetChangeAction")?,
                    }
                }
            }
            ActionGroup::General(actions) => match &actions.actions {
                GeneralAction::InitBridgeAccount(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for InitBridgeAccountAction")?,
                GeneralAction::BridgeSudoChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for BridgeSudoChangeAction")?,
            },
            ActionGroup::Sudo(actions) => match &actions.actions {
                SudoAction::SudoAddressChange(act) => act
                    .check_stateless()
                    .await
                    .context("stateless check failed for SudoAddressChangeAction")?,
            }, // No actions to check for Sudo
        }
        Ok(())
    }

    // allowed / FIXME: because most lines come from delegating (and error wrapping) to the
    // individual actions. This could be tidied up by implementing `ActionHandler for Action`
    // and letting it delegate.
    #[allow(clippy::too_many_lines)]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> anyhow::Result<()> {
        // Add the current signed transaction into the ephemeral state in case
        // downstream actions require access to it.
        // XXX: This must be deleted at the end of `check_stateful`.
        let mut transaction_context = state.put_transaction_context(self);

        // Transactions must match the chain id of the node.
        let chain_id = state.get_chain_id().await?;
        ensure!(
            self.chain_id() == chain_id.as_str(),
            InvalidChainId(self.chain_id().to_string())
        );

        // Nonce should be equal to the number of executed transactions before this tx.
        // First tx has nonce 0.
        let curr_nonce = state
            .get_account_nonce(self.address_bytes())
            .await
            .context("failed to get nonce for transaction signer")?;
        ensure!(curr_nonce == self.nonce(), InvalidNonce(self.nonce()));

        // Should have enough balance to cover all actions.
        check_balance_for_total_fees_and_transfers(self, &state)
            .await
            .context("failed to check balance for total fees and transfers")?;

        if state
            .get_bridge_account_rollup_id(self)
            .await
            .context("failed to check account rollup id")?
            .is_some()
        {
            state.put_last_transaction_id_for_bridge_account(
                self,
                &transaction_context.transaction_id,
            );
        }

        let from_nonce = state
            .get_account_nonce(self)
            .await
            .context("failed getting nonce of transaction signer")?;
        let next_nonce = from_nonce
            .checked_add(1)
            .context("overflow occurred incrementing stored nonce")?;
        state
            .put_account_nonce(self, next_nonce)
            .context("failed updating `from` nonce")?;

        // FIXME: this should create one span per `check_and_execute`
        match self.actions() {
            ActionGroup::BundlableGeneral(actions) => {
                for (i, action) in actions.actions.iter().enumerate() {
                    transaction_context.source_action_index = i as u64;
                    state.put_transaction_context(transaction_context);

                    match action {
                        BundlableGeneralAction::Transfer(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing transfer action failed")?,
                        BundlableGeneralAction::Sequence(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing sequence action failed")?,
                        BundlableGeneralAction::ValidatorUpdate(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing validator update failed")?,
                        BundlableGeneralAction::Ibc(act) => {
                            ensure!(
                                state
                                    .is_ibc_relayer(self)
                                    .await
                                    .context("failed to check if address is IBC relayer")?,
                                "only IBC sudo address can execute IBC actions"
                            );
                            let action = act
                                .clone()
                                .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                            action
                                .check_and_execute(&mut state)
                                .await
                                .context("failed executing ibc action")?;
                        }
                        BundlableGeneralAction::Ics20Withdrawal(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("failed executing ics20 withdrawal")?,
                        BundlableGeneralAction::BridgeLock(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("failed executing bridge lock")?,
                        BundlableGeneralAction::BridgeUnlock(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("failed executing bridge unlock")?,
                    }
                }
            }
            ActionGroup::General(actions) => {
                transaction_context.source_action_index = 0;
                state.put_transaction_context(transaction_context);

                match &actions.actions {
                    GeneralAction::InitBridgeAccount(act) => act
                        .check_and_execute(&mut state)
                        .await
                        .context("failed executing init bridge account")?,
                    GeneralAction::BridgeSudoChange(act) => act
                        .check_and_execute(&mut state)
                        .await
                        .context("failed executing bridge sudo change")?,
                }
            }
            ActionGroup::BundlableSudo(actions) => {
                for (i, action) in actions.actions.iter().enumerate() {
                    transaction_context.source_action_index = i as u64;
                    state.put_transaction_context(transaction_context);

                    match action {
                        BundlableSudoAction::FeeChange(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing fee change failed")?,
                        BundlableSudoAction::IbcRelayerChange(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing ibc relayer change failed")?,
                        BundlableSudoAction::FeeAssetChange(act) => act
                            .check_and_execute(&mut state)
                            .await
                            .context("executing fee asset change failed")?,
                    }
                }
            }
            ActionGroup::Sudo(actions) => {
                transaction_context.source_action_index = 0;
                state.put_transaction_context(transaction_context);

                match &actions.actions {
                    SudoAction::SudoAddressChange(act) => {
                        act.check_and_execute(&mut state)
                            .await
                            .context("failed executing sudo address change")?;
                    }
                }
            }
        }

        // XXX: Delete the current transaction data from the ephemeral state.
        state.delete_current_transaction_context();
        Ok(())
    }
}
