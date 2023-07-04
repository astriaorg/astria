use astria_proto::sequencer::v1::SequenceAction as ProtoSequenceAction;
use serde::{
    Deserialize,
    Serialize,
};

use crate::transaction::action_handler::ActionHandler;

/// Represents an opaque transaction destined for a rollup.
/// It only contains the chain ID of the destination rollup and data
/// which are bytes to be interpreted by the rollup.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Action {
    chain_id: Vec<u8>,
    data: Vec<u8>,
}

impl Action {
    #[allow(dead_code)]
    pub(crate) fn new(chain_id: Vec<u8>, data: Vec<u8>) -> Self {
        Self {
            chain_id,
            data,
        }
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
impl ActionHandler for Action {}
