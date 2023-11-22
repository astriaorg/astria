use std::{
    error::Error,
    fmt::Display,
};

use super::{
    asset,
    Address,
    IncorrectAddressLength,
};
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::sequencer::v1alpha1::asset::IncorrectAssetIdLength,
};

#[derive(Clone, Debug)]
pub struct TransferAction {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset_id: asset::Id,
}

impl TransferAction {
    #[must_use]
    pub fn into_raw(self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
            asset_id: asset_id.as_bytes().to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some((*amount).into()),
            asset_id: asset_id.as_bytes().to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::TransferAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `to` address did not have the expected
    /// length.
    pub fn try_from_raw(proto: raw::TransferAction) -> Result<Self, TransferActionError> {
        let raw::TransferAction {
            to,
            amount,
            asset_id,
        } = proto;
        let to = Address::try_from_slice(&to).map_err(TransferActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        let asset_id =
            asset::Id::try_from_slice(&asset_id).map_err(TransferActionError::asset_id)?;

        Ok(Self {
            to,
            amount,
            asset_id,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct TransferActionError {
    kind: TransferActionErrorKind,
}

impl TransferActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: TransferActionErrorKind::Address(inner),
        }
    }

    fn asset_id(inner: IncorrectAssetIdLength) -> Self {
        Self {
            kind: TransferActionErrorKind::Asset(inner),
        }
    }
}

impl Display for TransferActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TransferActionErrorKind::Address(_) => {
                f.pad("`to` field did not contain a valid address")
            }
            TransferActionErrorKind::Asset(_) => {
                f.pad("`asset_id` field did not contain a valid asset ID")
            }
        }
    }
}

impl Error for TransferActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            TransferActionErrorKind::Address(e) => Some(e),
            TransferActionErrorKind::Asset(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum TransferActionErrorKind {
    Address(IncorrectAddressLength),
    Asset(IncorrectAssetIdLength),
}
