use anyhow::{
    Context,
    Result,
};
use astria_proto::generated::sequencer::v1alpha1::{
    action::Value as ProtoValue,
    Action as ProtoAction,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::Transfer,
    sequence,
};

/// Represents an action on a specific module.
///
/// This type wraps all the different module-specific actions.
/// If a new action type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Action {
    TransferAction(Transfer),
    SequenceAction(sequence::Action),
}

impl Action {
    #[must_use]
    pub fn as_sequence(&self) -> Option<&sequence::Action> {
        match self {
            Self::SequenceAction(a) => Some(a),
            _ => None,
        }
    }

    #[must_use]
    pub fn new_sequence_action(chain_id: Vec<u8>, data: Vec<u8>) -> Self {
        Self::SequenceAction(sequence::Action::new(chain_id, data))
    }

    pub(crate) fn to_proto(&self) -> ProtoAction {
        match &self {
            Action::TransferAction(tx) => ProtoAction {
                value: Some(ProtoValue::TransferAction(tx.to_proto())),
            },
            Action::SequenceAction(tx) => ProtoAction {
                value: Some(ProtoValue::SequenceAction(tx.to_proto())),
            },
        }
    }

    pub(crate) fn try_from_proto(proto: &ProtoAction) -> Result<Self> {
        Ok(
            match proto
                .value
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing value"))?
            {
                ProtoValue::TransferAction(tx) => Action::TransferAction(
                    Transfer::try_from_proto(tx)
                        .context("failed to convert proto to TransferAction")?,
                ),
                ProtoValue::SequenceAction(tx) => {
                    Action::SequenceAction(sequence::Action::from_proto(tx))
                }
            },
        )
    }
}
