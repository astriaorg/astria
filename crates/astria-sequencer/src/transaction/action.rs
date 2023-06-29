use anyhow::Result;
use astria_proto::sequencer::v1::{
    action::Value::{
        SecondaryAction as ProtoSecondaryAction,
        Transfer as ProtoTransfer,
    },
    Action as ProtoAction,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::action::Transfer,
    secondary::action::Action as SecondaryAction,
};

/// Represents an action on a specific module.
/// This type wraps all the different module-specific actions.
/// If a new action type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Action {
    AccountsAction(Transfer),
    SecondaryAction(SecondaryAction),
}

impl Action {
    pub(crate) fn to_proto(&self) -> ProtoAction {
        match &self {
            Action::AccountsAction(tx) => ProtoAction {
                value: Some(ProtoTransfer(tx.to_proto())),
            },
            Action::SecondaryAction(tx) => ProtoAction {
                value: Some(ProtoSecondaryAction(tx.to_proto())),
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
                ProtoTransfer(tx) => Action::AccountsAction(Transfer::try_from_proto(tx)?),
                ProtoSecondaryAction(tx) => {
                    Action::SecondaryAction(SecondaryAction::from_proto(tx))
                }
            },
        )
    }
}
