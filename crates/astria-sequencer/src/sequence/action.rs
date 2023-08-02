use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_proto::sequencer::v1alpha1::SequenceAction as ProtoSequenceAction;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

use crate::{
    accounts::{
        state_ext::{
            StateReadExt,
            StateWriteExt,
        },
        types::{
            Address,
            Balance,
        },
    },
    transaction::action_handler::ActionHandler,
};

/// Fee charged for a sequence `Action` per byte of `data` included.
pub(crate) const SEQUENCE_ACTION_FEE_PER_BYTE: Balance = Balance(1);

/// Represents an opaque transaction destined for a rollup.
/// It only contains the chain ID of the destination rollup and data
/// which are bytes to be interpreted by the rollup.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Action {
    pub(crate) chain_id: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

impl Action {
    #[must_use]
    pub fn new(chain_id: Vec<u8>, data: Vec<u8>) -> Self {
        Self {
            chain_id,
            data,
        }
    }

    #[must_use]
    pub fn chain_id(&self) -> &[u8] {
        &self.chain_id
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn to_proto(&self) -> ProtoSequenceAction {
        ProtoSequenceAction {
            chain_id: self.chain_id.clone(),
            data: self.data.clone(),
        }
    }

    pub(crate) fn from_proto(proto: &ProtoSequenceAction) -> Self {
        Self {
            chain_id: proto.chain_id.clone(),
            data: proto.data.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ActionHandler for Action {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: &Address,
    ) -> Result<()> {
        let curr_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        let fee = SEQUENCE_ACTION_FEE_PER_BYTE * self.data.len() as u128;

        ensure!(curr_balance >= fee, "insufficient funds");

        Ok(())
    }

    fn check_stateless(&self) -> Result<()> {
        // TODO: do we want to place a maximum on the size of the data?
        ensure!(!self.data.is_empty(), "cannot have empty data");
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            from = from.to_string(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: &Address) -> Result<()> {
        let fee = SEQUENCE_ACTION_FEE_PER_BYTE * self.data.len() as u128;
        let from_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        state
            .put_account_balance(from, from_balance - fee)
            .context("failed updating `from` account balance")?;
        Ok(())
    }
}
