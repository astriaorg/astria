pub(crate) mod action_handler;

use std::fmt;

pub(crate) use action_handler::ActionHandler;
use anyhow::{
    ensure,
    Context as _,
};
use proto::native::sequencer::v1alpha1::{
    Action,
    Address,
    SignedTransaction,
    UnsignedTransaction,
};
use tracing::instrument;

use crate::accounts::state_ext::{
    StateReadExt,
    StateWriteExt,
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
        tx.unsigned_transaction().nonce >= curr_nonce,
        "nonce already used by account"
    );
    Ok(())
}

pub(crate) fn check_stateless(tx: &SignedTransaction) -> anyhow::Result<()> {
    tx.unsigned_transaction()
        .check_stateless()
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
    fn check_stateless(&self) -> anyhow::Result<()> {
        ensure!(!self.actions.is_empty(), "must have at least one action");

        for action in &self.actions {
            match action {
                Action::Transfer(act) => act
                    .check_stateless()
                    .context("stateless check failed for TransferAction")?,
                Action::Sequence(act) => act
                    .check_stateless()
                    .context("stateless check failed for SequenceAction")?,
                Action::ValidatorUpdate(act) => act
                    .check_stateless()
                    .context("stateless check failed for ValidatorUpdateAction")?,
            }
        }
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> anyhow::Result<()> {
        // Nonce should be equal to the number of executed transactions before this tx.
        // First tx has nonce 0.
        let curr_nonce = state.get_account_nonce(from).await?;
        ensure!(curr_nonce == self.nonce, InvalidNonce(self.nonce));

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
            }
        }

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            nonce = self.nonce,
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
            }
        }

        Ok(())
    }
}
