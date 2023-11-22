use std::{
    error::Error,
    fmt::Display,
};

use super::{
    Address,
    IncorrectAddressLength,
};
use crate::generated::sequencer::v1alpha1 as raw;

#[derive(Clone, Debug)]
pub struct MintAction {
    pub to: Address,
    pub amount: u128,
}

impl MintAction {
    #[must_use]
    pub fn into_raw(self) -> raw::MintAction {
        let Self {
            to,
            amount,
        } = self;
        raw::MintAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::MintAction {
        let Self {
            to,
            amount,
        } = self;
        raw::MintAction {
            to: to.to_vec(),
            amount: Some((*amount).into()),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::MintAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `to` address did not have the expected
    /// length.
    pub fn try_from_raw(proto: raw::MintAction) -> Result<Self, MintActionError> {
        let raw::MintAction {
            to,
            amount,
        } = proto;
        let to = Address::try_from_slice(&to).map_err(MintActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        Ok(Self {
            to,
            amount,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct MintActionError {
    kind: MintActionErrorKind,
}

impl MintActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: MintActionErrorKind::Address(inner),
        }
    }
}

impl Display for MintActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            MintActionErrorKind::Address(_) => f.pad("`to` field did not contain a valid address"),
        }
    }
}

impl Error for MintActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            MintActionErrorKind::Address(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum MintActionErrorKind {
    Address(IncorrectAddressLength),
}
