use anyhow::Result;
use astria_proto::sequencer::v1::{
    action::Value::{
        SecondaryAction as ProtoSecondaryTransaction,
        Transfer as ProtoAccountsTransaction,
    },
    Action as ProtoAction,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    accounts::transaction::Transfer as AccountsTransaction,
    secondary::transaction::Transaction as SecondaryTransaction,
};

/// Represents an action on a specific module.
/// This type wraps all the different module-specific actions.
/// If a new action type is added, it should be added to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum Action {
    AccountsAction(AccountsTransaction),
    SecondaryAction(SecondaryTransaction),
}

impl Action {
    pub(crate) fn to_proto(&self) -> ProtoAction {
        match &self {
            Action::AccountsAction(tx) => ProtoAction {
                value: Some(ProtoAccountsTransaction(tx.to_proto())),
            },
            Action::SecondaryAction(tx) => ProtoAction {
                value: Some(ProtoSecondaryTransaction(tx.to_proto())),
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
                ProtoAccountsTransaction(tx) => {
                    Action::AccountsAction(AccountsTransaction::try_from_proto(tx)?)
                }
                ProtoSecondaryTransaction(tx) => {
                    Action::SecondaryAction(SecondaryTransaction::from_proto(tx))
                }
            },
        )
    }
}
