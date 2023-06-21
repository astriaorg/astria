use anyhow::{
    ensure,
    Context,
    Result,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

use crate::accounts::{
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    types::{
        Address,
        Nonce,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transaction {
    nonce: Nonce,
    data: Vec<u8>,
}

impl Transaction {
    pub(crate) fn to_proto(&self) -> astria_proto::sequencer::v1::SecondaryTransaction {
        astria_proto::sequencer::v1::SecondaryTransaction {
            nonce: self.nonce.into(),
            data: self.data.clone(),
        }
    }

    pub(crate) fn from_proto(
        proto: &astria_proto::sequencer::v1::SecondaryTransaction,
    ) -> Result<Self> {
        Ok(Self {
            nonce: Nonce::from(proto.nonce),
            data: proto.data.clone(),
        })
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    pub(crate) fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: &Address,
    ) -> Result<()> {
        let curr_nonce = state.get_account_nonce(from).await?;
        ensure!(curr_nonce < self.nonce, "invalid nonce");
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            nonce = self.nonce.into_inner(),
        )
    )]
    pub(crate) async fn execute<S: StateWriteExt>(
        &self,
        state: &mut S,
        from: &Address,
    ) -> Result<()> {
        let from_nonce = state
            .get_account_nonce(from)
            .await
            .context("failed getting `from` nonce")?;
        state
            .put_account_nonce(from, from_nonce + Nonce::from(1))
            .context("failed updating `from` nonce")?;
        Ok(())
    }
}
