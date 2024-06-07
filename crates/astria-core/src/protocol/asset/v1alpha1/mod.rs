use super::raw;
use crate::primitive::v1::asset::Denom;

/// The sequencer response to a denomination request for a given asset ID.
#[derive(Clone, Debug, PartialEq)]
pub struct DenomResponse {
    pub height: u64,
    pub denom: Denom,
}

impl DenomResponse {
    /// Converts a protobuf [`raw::DenomResponse`] to an astria
    /// native [`DenomResponse`].
    #[must_use]
    pub fn from_raw(proto: &raw::DenomResponse) -> Self {
        let raw::DenomResponse {
            height,
            denom,
        } = proto;
        Self {
            height: *height,
            denom: denom.clone().into(),
        }
    }

    /// Converts an astria native [`DenomResponse`] to a
    /// protobuf [`raw::DenomResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::DenomResponse {
        raw::DenomResponse::from_native(self)
    }
}

impl raw::DenomResponse {
    /// Converts an astria native [`DenomResponse`] to a
    /// protobuf [`raw::DenomResponse`].
    #[must_use]
    pub fn from_native(native: DenomResponse) -> Self {
        let DenomResponse {
            height,
            denom,
        } = native;
        Self {
            height,
            denom: denom.to_string(),
        }
    }

    /// Converts a protobuf [`raw::DenomResponse`] to an astria
    /// native [`DenomResponse`].
    #[must_use]
    pub fn into_native(self) -> DenomResponse {
        DenomResponse::from_raw(&self)
    }

    /// Converts a protobuf [`raw::DenomResponse`] to an astria
    /// native [`DenomResponse`] by allocating a new [`v1alpha1::DenomResponse`].
    #[must_use]
    pub fn to_native(&self) -> DenomResponse {
        self.clone().into_native()
    }
}
