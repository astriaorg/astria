use super::raw;
use crate::primitive::v1::asset::{
    self,
    Denom,
    ParseDenomError,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct DenomResponseError(DenomResponseErrorKind);
impl DenomResponseError {
    #[must_use]
    fn invalid_denom(source: ParseDenomError) -> Self {
        Self(DenomResponseErrorKind::InvalidDenom {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum DenomResponseErrorKind {
    #[error("`denom` field was invalid")]
    InvalidDenom { source: ParseDenomError },
}

/// The sequencer response to a denomination request for a given asset ID.
#[derive(Clone, Debug, PartialEq)]
pub struct DenomResponse {
    pub height: u64,
    pub denom: Denom,
}

impl DenomResponse {
    /// Converts a protobuf [`raw::DenomResponse`] to an astria
    /// native [`DenomResponse`].
    ///
    /// # Errors
    /// Returns an error if the `denom` field  of the proto file can't be parsed as a [`Denom`].
    pub fn try_from_raw(proto: &raw::DenomResponse) -> Result<Self, DenomResponseError> {
        let raw::DenomResponse {
            height,
            denom,
        } = proto;
        Ok(Self {
            height: *height,
            denom: denom.parse().map_err(DenomResponseError::invalid_denom)?,
        })
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

    // /// Converts a protobuf [`raw::DenomResponse`] to an astria
    // /// native [`DenomResponse`].
    // #[must_use]
    // pub fn try_into_native(self) -> Result<DenomResponse, DenomResponseError> {
    //     DenomResponse::try_from_raw(&self)
    // }

    // /// Converts a protobuf [`raw::DenomResponse`] to an astria
    // /// native [`DenomResponse`] by allocating a new [`v1alpha1::DenomResponse`].
    // #[must_use]
    // pub fn to_native(&self) -> Result<DenomResponse, DenomResponseError> {
    //     DenomResponse::try_from_raw(self)
    // }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AllowedFeeAssetsResponseError(AllowedFeeAssetsResponseErrorKind);

#[derive(Debug, thiserror::Error)]
enum AllowedFeeAssetsResponseErrorKind {
    #[error("failed to parse assets as IBC ICS20 denom")]
    IncorrectAsset(#[source] ParseDenomError),
}

impl AllowedFeeAssetsResponseError {
    fn incorrect_asset(inner: ParseDenomError) -> Self {
        Self(AllowedFeeAssetsResponseErrorKind::IncorrectAsset(inner))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AllowedFeeAssetsResponse {
    pub height: u64,
    pub fee_assets: Vec<asset::Denom>,
}

impl AllowedFeeAssetsResponse {
    /// Converts a protobuf [`raw::AllowedFeeAssetsResponse`] to an astria
    /// native [`AllowedFeeAssetsResponse`].
    ///
    /// # Errors
    /// - If one of the fee asset strings cannot be parsed as an [`asset::Denom`].
    pub fn try_from_raw(
        proto: &raw::AllowedFeeAssetsResponse,
    ) -> Result<Self, AllowedFeeAssetsResponseError> {
        let raw::AllowedFeeAssetsResponse {
            height,
            fee_assets,
        } = proto;
        let mut assets: Vec<asset::Denom> = Vec::new();

        for s in fee_assets {
            let native = s
                .parse()
                .map_err(AllowedFeeAssetsResponseError::incorrect_asset)?;
            assets.push(native);
        }

        Ok(Self {
            height: *height,
            fee_assets: assets,
        })
    }

    /// Converts an astria native [`AllowedFeeAssetsResponse`] to a
    /// protobuf [`raw::AllowedFeeAssetsResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::AllowedFeeAssetsResponse {
        raw::AllowedFeeAssetsResponse::from_native(self)
    }
}

impl raw::AllowedFeeAssetsResponse {
    /// Converts an astria native [`AllowedFeeAssetsResponse`] to a
    /// protobuf [`raw::AllowedFeeAssetsResponse`].
    #[must_use]
    pub fn from_native(native: AllowedFeeAssetsResponse) -> Self {
        let AllowedFeeAssetsResponse {
            height,
            fee_assets,
        } = native;
        let fee_assets = fee_assets
            .into_iter()
            .map(|denom| denom.to_string())
            .collect();
        Self {
            height,
            fee_assets,
        }
    }

    /// Converts a protobuf [`raw::AllowedFeeAssetsResponse`] to an astria
    /// native [`AllowedFeeAssetsResponse`].
    ///
    /// # Errors
    /// - If one of the assets  cannot be parsed as an [`asset::Denom`].
    pub fn try_into_native(
        self,
    ) -> Result<AllowedFeeAssetsResponse, AllowedFeeAssetsResponseError> {
        AllowedFeeAssetsResponse::try_from_raw(&self)
    }

    /// Converts a protobuf [`raw::AllowedFeeAssetsResponse`] to an astria
    /// native [`AllowedFeeAssetsResponse`] by allocating a new
    /// [`v1alpha1::AllowedFeeAssetsResponse`].
    ///
    /// # Errors
    /// - If one of the assets  cannot be parsed as an [`asset::Denom`].
    pub fn try_to_native(&self) -> Result<AllowedFeeAssetsResponse, AllowedFeeAssetsResponseError> {
        self.clone().try_into_native()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denom_response_from_raw_is_correct() {
        let raw = raw::DenomResponse {
            height: 42,
            denom: "nria".to_owned(),
        };
        let expected = DenomResponse {
            height: 42,
            denom: "nria".parse().unwrap(),
        };
        let actual = DenomResponse::try_from_raw(&raw).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn denom_response_into_raw_is_correct() {
        let native = DenomResponse {
            height: 42,
            denom: "nria".parse().unwrap(),
        };
        let expected = raw::DenomResponse {
            height: 42,
            denom: "nria".to_owned(),
        };
        let actual = native.into_raw();
        assert_eq!(expected, actual);
    }

    #[test]
    fn denom_response_roundtrip_is_correct() {
        let native = DenomResponse {
            height: 42,
            denom: "nria".parse().unwrap(),
        };
        let expected = native.clone();
        let actual = DenomResponse::try_from_raw(&native.into_raw()).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn allowed_fee_assets_try_from_raw_is_correct() {
        let raw = raw::AllowedFeeAssetsResponse {
            height: 42,
            fee_assets: vec![
                "asset_0".to_string(),
                "asset_1".to_string(),
                "asset_2".to_string(),
            ],
        };
        let expected = AllowedFeeAssetsResponse {
            height: 42,
            fee_assets: vec![
                "asset_0".parse().unwrap(),
                "asset_1".parse().unwrap(),
                "asset_2".parse().unwrap(),
            ],
        };
        let actual = AllowedFeeAssetsResponse::try_from_raw(&raw).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn allowed_fee_assets_into_raw_is_correct() {
        let native = AllowedFeeAssetsResponse {
            height: 42,
            fee_assets: vec![
                "asset_0".parse().unwrap(),
                "asset_1".parse().unwrap(),
                "asset_2".parse().unwrap(),
            ],
        };
        let expected = raw::AllowedFeeAssetsResponse {
            height: 42,
            fee_assets: vec![
                "asset_0".to_string(),
                "asset_1".to_string(),
                "asset_2".to_string(),
            ],
        };
        let actual = native.into_raw();
        assert_eq!(expected, actual);
    }

    #[test]
    fn allowed_fee_assets_roundtrip_is_correct() {
        let native = AllowedFeeAssetsResponse {
            height: 42,
            fee_assets: vec![
                "asset_0".parse().unwrap(),
                "asset_1".parse().unwrap(),
                "asset_2".parse().unwrap(),
            ],
        };
        let expected = native.clone();
        let actual = native.into_raw().try_into_native().unwrap();
        assert_eq!(expected, actual);
    }
}
