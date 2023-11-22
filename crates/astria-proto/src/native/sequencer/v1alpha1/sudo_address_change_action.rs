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
pub struct SudoAddressChangeAction {
    pub new_address: Address,
}

impl SudoAddressChangeAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: new_address.to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: new_address.to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SudoAddressChangeAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length.
    pub fn try_from_raw(
        proto: raw::SudoAddressChangeAction,
    ) -> Result<Self, SudoAddressChangeActionError> {
        let raw::SudoAddressChangeAction {
            new_address,
        } = proto;
        let new_address =
            Address::try_from_slice(&new_address).map_err(SudoAddressChangeActionError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct SudoAddressChangeActionError {
    kind: SudoAddressChangeActionErrorKind,
}

impl SudoAddressChangeActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: SudoAddressChangeActionErrorKind::Address(inner),
        }
    }
}

impl Display for SudoAddressChangeActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SudoAddressChangeActionErrorKind::Address(_) => {
                f.pad("`new_address` field did not contain a valid address")
            }
        }
    }
}

impl Error for SudoAddressChangeActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SudoAddressChangeActionErrorKind::Address(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SudoAddressChangeActionErrorKind {
    Address(IncorrectAddressLength),
}
