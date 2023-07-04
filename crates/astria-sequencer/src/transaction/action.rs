use anyhow::{
    Context,
    Result,
};
use astria_proto::sequencer::v1::{
    action::Value as ProtoValue,
    Action as ProtoAction,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::TransferAction,
    sequence,
};

/// Represents an action on a specific module.
///
/// This type wraps all the different module-specific actions.
/// If a new action type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Action {
    TransferAction(TransferAction),
    SequenceAction(sequence::Action),
}

impl Action {
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
                    TransferAction::try_from_proto(tx)
                        .context("failed to convert proto to TransferAction")?,
                ),
                ProtoValue::SequenceAction(tx) => {
                    Action::SequenceAction(sequence::Action::from_proto(tx))
                }
            },
        )
    }
}
