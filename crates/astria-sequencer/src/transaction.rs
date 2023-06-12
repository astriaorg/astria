use anyhow::Result;
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use serde::{
    Deserialize,
    Serialize,
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
/// If a new transaction type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Transaction {
    AccountsTransaction(AccountsTransaction),
}

impl Transaction {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tx = serde_json::from_slice(bytes)?;
        Ok(tx)
    }
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
