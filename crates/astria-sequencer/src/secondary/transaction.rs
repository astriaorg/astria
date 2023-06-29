use anyhow::Result;
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
        types::Address,
    },
    transaction::action_handler::ActionHandler,
};

/// Represents an opaque transaction destined for a rollup.
/// It only contains the chain ID of the destination rollup and data
/// which are bytes to be interpreted by the rollup.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Transaction {
    chain_id: Vec<u8>,
    data: Vec<u8>,
}

impl Transaction {
    #[allow(dead_code)]
    pub(crate) fn new(chain_id: Vec<u8>, data: Vec<u8>) -> Self {
        Self {
            chain_id,
            data,
        }
    }

    pub(crate) fn to_proto(&self) -> astria_proto::sequencer::v1::SecondaryAction {
        astria_proto::sequencer::v1::SecondaryAction {
            chain_id: self.chain_id.clone(),
            data: self.data.clone(),
        }
    }

    pub(crate) fn from_proto(proto: &astria_proto::sequencer::v1::SecondaryAction) -> Self {
        Self {
            chain_id: proto.chain_id.clone(),
            data: proto.data.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ActionHandler for Transaction {
    fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        _state: &S,
        _from: &Address,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, _state: &mut S, _from: &Address) -> Result<()> {
        Ok(())
    }
}
