use std::{
    error::Error,
    fmt::Display,
};

use super::{
    MintAction,
    MintActionError,
    SequenceAction,
    SequenceActionError,
    SudoAddressChangeAction,
    SudoAddressChangeActionError,
    TransferAction,
    TransferActionError,
};
use crate::generated::sequencer::v1alpha1 as raw;

#[derive(Clone, Debug)]
pub enum Action {
    Sequence(SequenceAction),
    Transfer(TransferAction),
    ValidatorUpdate(tendermint::validator::Update),
    SudoAddressChange(SudoAddressChangeAction),
    Mint(MintAction),
}

impl Action {
    #[must_use]
    pub fn into_raw(self) -> raw::Action {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.into_raw()),
            Action::Transfer(act) => Value::TransferAction(act.into_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdateAction(act.into()),
            Action::SudoAddressChange(act) => Value::SudoAddressChangeAction(act.into_raw()),
            Action::Mint(act) => Value::MintAction(act.into_raw()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::Action {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.to_raw()),
            Action::Transfer(act) => Value::TransferAction(act.to_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdateAction(act.clone().into()),
            Action::SudoAddressChange(act) => {
                Value::SudoAddressChangeAction(act.clone().into_raw())
            }
            Action::Mint(act) => Value::MintAction(act.to_raw()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Action`].
    ///
    /// # Errors
    ///
    /// Returns an error if conversion of one of the inner raw action variants
    /// to a native action ([`SequenceAction`] or [`TransferAction`]) fails.
    pub fn try_from_raw(proto: raw::Action) -> Result<Self, ActionError> {
        use raw::action::Value;
        let raw::Action {
            value,
        } = proto;
        let Some(action) = value else {
            return Err(ActionError::unset());
        };
        let action = match action {
            Value::SequenceAction(act) => {
                Self::Sequence(SequenceAction::try_from_raw(act).map_err(ActionError::sequence)?)
            }
            Value::TransferAction(act) => {
                Self::Transfer(TransferAction::try_from_raw(act).map_err(ActionError::transfer)?)
            }
            Value::ValidatorUpdateAction(act) => {
                Self::ValidatorUpdate(act.try_into().map_err(ActionError::validator_update)?)
            }
            Value::SudoAddressChangeAction(act) => Self::SudoAddressChange(
                SudoAddressChangeAction::try_from_raw(act)
                    .map_err(ActionError::sudo_address_change)?,
            ),
            Value::MintAction(act) => {
                Self::Mint(MintAction::try_from_raw(act).map_err(ActionError::mint)?)
            }
        };
        Ok(action)
    }

    #[must_use]
    pub fn as_sequence(&self) -> Option<&SequenceAction> {
        let Self::Sequence(sequence_action) = self else {
            return None;
        };
        Some(sequence_action)
    }

    #[must_use]
    pub fn as_transfer(&self) -> Option<&TransferAction> {
        let Self::Transfer(transfer_action) = self else {
            return None;
        };
        Some(transfer_action)
    }
}

impl From<SequenceAction> for Action {
    fn from(value: SequenceAction) -> Self {
        Self::Sequence(value)
    }
}

impl From<TransferAction> for Action {
    fn from(value: TransferAction) -> Self {
        Self::Transfer(value)
    }
}

impl From<SudoAddressChangeAction> for Action {
    fn from(value: SudoAddressChangeAction) -> Self {
        Self::SudoAddressChange(value)
    }
}

impl From<MintAction> for Action {
    fn from(value: MintAction) -> Self {
        Self::Mint(value)
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ActionError {
    kind: ActionErrorKind,
}

impl ActionError {
    fn unset() -> Self {
        Self {
            kind: ActionErrorKind::Unset,
        }
    }

    fn sequence(inner: SequenceActionError) -> Self {
        Self {
            kind: ActionErrorKind::Sequence(inner),
        }
    }

    fn transfer(inner: TransferActionError) -> Self {
        Self {
            kind: ActionErrorKind::Transfer(inner),
        }
    }

    fn validator_update(inner: tendermint::error::Error) -> Self {
        Self {
            kind: ActionErrorKind::ValidatorUpdate(inner),
        }
    }

    fn sudo_address_change(inner: SudoAddressChangeActionError) -> Self {
        Self {
            kind: ActionErrorKind::SudoAddressChange(inner),
        }
    }

    fn mint(inner: MintActionError) -> Self {
        Self {
            kind: ActionErrorKind::Mint(inner),
        }
    }
}

impl Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self.kind {
            ActionErrorKind::Unset => "oneof value was not set",
            ActionErrorKind::Sequence(_) => "raw sequence action was not valid",
            ActionErrorKind::Transfer(_) => "raw transfer action was not valid",
            ActionErrorKind::ValidatorUpdate(_) => "raw validator update action was not valid",
            ActionErrorKind::SudoAddressChange(_) => "raw sudo address change action was not valid",
            ActionErrorKind::Mint(_) => "raw mint action was not valid",
        };
        f.pad(msg)
    }
}

impl Error for ActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            ActionErrorKind::Unset => None,
            ActionErrorKind::Sequence(e) => Some(e),
            ActionErrorKind::Transfer(e) => Some(e),
            ActionErrorKind::ValidatorUpdate(e) => Some(e),
            ActionErrorKind::SudoAddressChange(e) => Some(e),
            ActionErrorKind::Mint(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum ActionErrorKind {
    Unset,
    Sequence(SequenceActionError),
    Transfer(TransferActionError),
    ValidatorUpdate(tendermint::error::Error),
    SudoAddressChange(SudoAddressChangeActionError),
    Mint(MintActionError),
}
