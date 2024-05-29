use super::raw;
use crate::primitive::v1::asset::{
    self,
    Denom,
    IncorrectAssetIdLength,
};

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

#[derive(Debug, PartialEq)]
pub struct AllowedFeeAssetsResponse {
    pub height: u64,
    pub fee_asset_ids: Vec<asset::Id>,
}

impl AllowedFeeAssetsResponse {
    pub fn try_from_raw(proto: &raw::FeeAssetsResponse) -> Result<Self, IncorrectAssetIdLength> {
        let raw::FeeAssetsResponse {
            height,
            fee_asset_ids,
        } = proto;
        let mut assets: Vec<asset::Id> = Vec::new();

        for raw_id in fee_asset_ids {
            let native_id = asset::Id::try_from_slice(&raw_id)?;
            assets.push(native_id);
        }

        Ok(Self {
            height: *height,
            fee_asset_ids: assets,
        })
    }

    pub fn into_raw(self) -> raw::FeeAssetsResponse {
        raw::FeeAssetsResponse::from_native(self)
    }
}

impl raw::FeeAssetsResponse {
    pub fn from_native(native: AllowedFeeAssetsResponse) -> Self {
        let AllowedFeeAssetsResponse {
            height,
            fee_asset_ids,
        } = native;
        let raw_assets = fee_asset_ids
            .into_iter()
            .map(|id| id.as_ref().into())
            .collect();
        Self {
            height,
            fee_asset_ids: raw_assets,
        }
    }

    pub fn try_into_native(self) -> Result<AllowedFeeAssetsResponse, IncorrectAssetIdLength> {
        AllowedFeeAssetsResponse::try_from_raw(&self)
    }

    pub fn try_to_native(&self) -> Result<AllowedFeeAssetsResponse, IncorrectAssetIdLength> {
        self.clone().try_into_native()
    }
}
