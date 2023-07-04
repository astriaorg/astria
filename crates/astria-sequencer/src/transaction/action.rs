use anyhow::Result;
use astria_proto::sequencer::v1::{
    action::Value::{
        SequenceAction as ProtoSequenceAction,
        TransferAction as ProtoTransferAction,
    },
    Action as ProtoAction,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::TransferAction,
    sequence::Action as SequenceAction,
};

/// Represents an action on a specific module.
/// This type wraps all the different module-specific actions.
/// If a new action type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Action {
    TransferAction(TransferAction),
    SequenceAction(SequenceAction),
}

impl Action {
    pub(crate) fn to_proto(&self) -> ProtoAction {
        match &self {
            Action::TransferAction(tx) => ProtoAction {
                value: Some(ProtoTransferAction(tx.to_proto())),
            },
            Action::SequenceAction(tx) => ProtoAction {
                value: Some(ProtoSequenceAction(tx.to_proto())),
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
                ProtoTransferAction(tx) => {
                    Action::TransferAction(TransferAction::try_from_proto(tx)?)
                }
                ProtoSequenceAction(tx) => Action::SequenceAction(SequenceAction::from_proto(tx)),
            },
        )
    }
}
