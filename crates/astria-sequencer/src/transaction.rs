use anyhow::Result;
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use crate::accounts::transaction::Transaction as AccountsTransaction;

#[async_trait]
pub(crate) trait ActionHandler {
    fn check_stateless(&self) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()>;
}

/// Represents a sequencer chain transaction.
///
/// If a new transaction type is added, it should be added to this enum.
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum Transaction {
    AccountsTransaction(AccountsTransaction),
}

#[async_trait]
impl ActionHandler for Transaction {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateless(),
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.check_stateful(state).await,
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match self {
            Self::AccountsTransaction(tx) => tx.execute(state).await,
        }
    }
}
