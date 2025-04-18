use std::fmt;

use astria_core::protocol::{
    fees::v1::FeeComponents,
    transaction::v1::{
        Action,
        Transaction,
    },
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ensure,
        Context as _,
        OptionExt as _,
        Report,
        Result,
    },
};
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    action_handler::ActionHandler,
    app::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    fees::FeeHandler,
    ibc::{
        host_interface::AstriaHost,
        StateReadExt as _,
    },
    storage::StoredValue,
    transaction::{
        check_balance_for_total_fees_and_transfers,
        StateWriteExt as _,
    },
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
impl ActionHandler for Transaction {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_stateless(&self) -> Result<()> {
        ensure!(!self.actions().is_empty(), "must have at least one action");

        for action in self.actions() {
            match action {
                Action::Transfer(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for Transfer action")?,
                Action::RollupDataSubmission(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for RollupDataSubmission action")?,
                Action::ValidatorUpdate(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for ValidatorUpdate action")?,
                Action::SudoAddressChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for SudoAddressChange action")?,
                Action::IbcSudoChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for IbcSudoChange action")?,
                Action::FeeChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for FeeChange action")?,
                Action::Ibc(act) => {
                    let action = act
                        .clone()
                        .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                    action
                        .check_stateless(())
                        .await
                        .map_err(anyhow_to_eyre)
                        .wrap_err("stateless check failed for Ibc action")?;
                }
                Action::Ics20Withdrawal(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for Ics20Withdrawal action")?,
                Action::IbcRelayerChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for IbcRelayerChange action")?,
                Action::FeeAssetChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for FeeAssetChange action")?,
                Action::InitBridgeAccount(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for InitBridgeAccount action")?,
                Action::BridgeLock(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for BridgeLock action")?,
                Action::BridgeUnlock(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for BridgeUnlock action")?,
                Action::BridgeTransfer(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for BridgeTransfer action")?,
                Action::BridgeSudoChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for BridgeSudoChange action")?,
                Action::RecoverIbcClient(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for RecoverIbcClient action")?,
                Action::CurrencyPairsChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for CurrencyPairsChange action")?,
                Action::MarketsChange(act) => act
                    .check_stateless()
                    .await
                    .wrap_err("stateless check failed for MarketsChange action")?,
            }
        }
        Ok(())
    }

    // FIXME (https://github.com/astriaorg/astria/issues/1584): because most lines come from delegating (and error wrapping) to the
    // individual actions. This could be tidied up by implementing `ActionHandler for Action`
    // and letting it delegate.
    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
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
            .wrap_err("failed to get nonce for transaction signer")?;
        ensure!(curr_nonce == self.nonce(), InvalidNonce(self.nonce()));

        // Should have enough balance to cover all actions.
        check_balance_for_total_fees_and_transfers(self, &state)
            .await
            .wrap_err("failed to check balance for total fees and transfers")?;

        if state
            .get_bridge_account_rollup_id(&self)
            .await
            .wrap_err("failed to check account rollup id")?
            .is_some()
        {
            state
                .put_last_transaction_id_for_bridge_account(
                    &self,
                    transaction_context.transaction_id,
                )
                .wrap_err("failed to put last transaction id for bridge account")?;
        }

        let from_nonce = state
            .get_account_nonce(&self)
            .await
            .wrap_err("failed getting nonce of transaction signer")?;
        let next_nonce = from_nonce
            .checked_add(1)
            .ok_or_eyre("overflow occurred incrementing stored nonce")?;
        state
            .put_account_nonce(&self, next_nonce)
            .wrap_err("failed updating `from` nonce")?;

        // FIXME: this should create one span per `check_and_execute`
        for (i, action) in (0..).zip(self.actions().iter()) {
            transaction_context.position_in_transaction = i;
            state.put_transaction_context(transaction_context);

            match action {
                Action::Transfer(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing transfer action failed")?,
                Action::RollupDataSubmission(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing sequence action failed")?,
                Action::ValidatorUpdate(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing validor update")?,
                Action::SudoAddressChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing sudo address change failed")?,
                Action::IbcSudoChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing ibc sudo change failed")?,
                Action::FeeChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("executing fee change failed")?,
                Action::Ibc(act) => {
                    // FIXME: this check should be moved to check_and_execute, as it now has
                    // access to the the signer through state. However, what's the correct
                    // ibc AppHandler call to do it? Can we just update one of the trait methods
                    // of crate::ibc::ics20_transfer::Ics20Transfer?
                    ensure!(
                        state
                            .is_ibc_relayer(self)
                            .await
                            .wrap_err("failed to check if address is IBC relayer")?,
                        "only IBC sudo address can execute IBC actions"
                    );
                    let action = act
                        .clone()
                        .with_handler::<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>();
                    action
                        .check_and_execute(&mut state)
                        .await
                        .map_err(anyhow_to_eyre)
                        .wrap_err("failed executing ibc action")?;
                }
                Action::Ics20Withdrawal(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing ics20 withdrawal")?,
                Action::IbcRelayerChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing ibc relayer change")?,
                Action::FeeAssetChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing fee asseet change")?,
                Action::InitBridgeAccount(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing init bridge account")?,
                Action::BridgeLock(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing bridge lock")?,
                Action::BridgeUnlock(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing bridge unlock")?,
                Action::BridgeTransfer(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing bridge transfer")?,
                Action::BridgeSudoChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing bridge sudo change")?,
                Action::RecoverIbcClient(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing recover ibc client")?,
                Action::CurrencyPairsChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing currency pairs change")?,
                Action::MarketsChange(act) => check_execute_and_pay_fees(act, &mut state)
                    .await
                    .wrap_err("failed executing markets change")?,
            }
        }

        // XXX: Delete the current transaction data from the ephemeral state.
        state.delete_current_transaction_context();
        Ok(())
    }
}

async fn check_execute_and_pay_fees<'a, T, S>(action: &T, mut state: S) -> Result<()>
where
    T: ActionHandler + FeeHandler + Sync,
    FeeComponents<T>: TryFrom<StoredValue<'a>, Error = Report>,
    S: StateWrite,
{
    action.check_and_execute(&mut state).await?;
    action.check_and_pay_fees(&mut state).await?;
    Ok(())
}
