use super::raw;
use crate::primitive::v1::asset::{
    self,
    Denom,
    IncorrectAssetIdLength,
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
pub struct AllowedFeeAssetIdsResponseError(AllowedFeeAssetIdsResponseErrorKind);

#[derive(Debug, thiserror::Error)]
enum AllowedFeeAssetIdsResponseErrorKind {
    #[error("failed to convert asset ID")]
    IncorrectAssetIdLength(#[source] IncorrectAssetIdLength),
}

impl AllowedFeeAssetIdsResponseError {
    fn incorrect_asset_id_length(inner: IncorrectAssetIdLength) -> Self {
        Self(AllowedFeeAssetIdsResponseErrorKind::IncorrectAssetIdLength(
            inner,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AllowedFeeAssetIdsResponse {
    pub height: u64,
    pub fee_asset_ids: Vec<asset::Id>,
}

impl AllowedFeeAssetIdsResponse {
    /// Converts a protobuf [`raw::AllowedFeeAssetIdsResponse`] to an astria
    /// native [`AllowedFeeAssetIdsResponse`].
    ///
    /// # Errors
    /// - If one of the serialized asset IDs cannot be converted to a [`asset::Id`].
    pub fn try_from_raw(
        proto: &raw::AllowedFeeAssetIdsResponse,
    ) -> Result<Self, AllowedFeeAssetIdsResponseError> {
        let raw::AllowedFeeAssetIdsResponse {
            height,
            fee_asset_ids,
        } = proto;
        let mut assets: Vec<asset::Id> = Vec::new();

        for raw_id in fee_asset_ids {
            let native_id = asset::Id::try_from_slice(raw_id)
                .map_err(AllowedFeeAssetIdsResponseError::incorrect_asset_id_length)?;
            assets.push(native_id);
        }

        Ok(Self {
            height: *height,
            fee_asset_ids: assets,
        })
    }

    /// Converts an astria native [`AllowedFeeAssetIdsResponse`] to a
    /// protobuf [`raw::AllowedFeeAssetIdsResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::AllowedFeeAssetIdsResponse {
        raw::AllowedFeeAssetIdsResponse::from_native(self)
    }
}

impl raw::AllowedFeeAssetIdsResponse {
    /// Converts an astria native [`AllowedFeeAssetIdsResponse`] to a
    /// protobuf [`raw::AllowedFeeAssetIdsResponse`].
    #[must_use]
    pub fn from_native(native: AllowedFeeAssetIdsResponse) -> Self {
        let AllowedFeeAssetIdsResponse {
            height,
            fee_asset_ids,
        } = native;
        let raw_assets = fee_asset_ids
            .into_iter()
            .map(|id| id.as_ref().to_vec().into())
            .collect();
        Self {
            height,
            fee_asset_ids: raw_assets,
        }
    }

    /// Converts a protobuf [`raw::AllowedFeeAssetIdsResponse`] to an astria
    /// native [`AllowedFeeAssetIdsResponse`].
    ///
    /// # Errors
    /// - If one of the serialized asset IDs cannot be converted to a [`asset::Id`].
    pub fn try_into_native(
        self,
    ) -> Result<AllowedFeeAssetIdsResponse, AllowedFeeAssetIdsResponseError> {
        AllowedFeeAssetIdsResponse::try_from_raw(&self)
    }

    /// Converts a protobuf [`raw::AllowedFeeAssetIdsResponse`] to an astria
    /// native [`AllowedFeeAssetIdsResponse`] by allocating a new
    /// [`v1alpha1::AllowedFeeAssetIdsResponse`].
    ///
    /// # Errors
    /// - If one of the serialized asset IDs cannot be converted to a [`asset::Id`].
    pub fn try_to_native(
        &self,
    ) -> Result<AllowedFeeAssetIdsResponse, AllowedFeeAssetIdsResponseError> {
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
    fn allowed_fee_asset_ids_try_from_raw_is_correct() {
        let raw = raw::AllowedFeeAssetIdsResponse {
            height: 42,
            fee_asset_ids: vec![
                asset::Id::from_str_unchecked("asset_0")
                    .get()
                    .to_vec()
                    .into(),
                asset::Id::from_str_unchecked("asset_1")
                    .get()
                    .to_vec()
                    .into(),
                asset::Id::from_str_unchecked("asset_2")
                    .get()
                    .to_vec()
                    .into(),
            ],
        };
        let expected = AllowedFeeAssetIdsResponse {
            height: 42,
            fee_asset_ids: vec![
                asset::Id::from_str_unchecked("asset_0"),
                asset::Id::from_str_unchecked("asset_1"),
                asset::Id::from_str_unchecked("asset_2"),
            ],
        };
        let actual = AllowedFeeAssetIdsResponse::try_from_raw(&raw).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn allowed_fee_asset_ids_into_raw_is_correct() {
        let native = AllowedFeeAssetIdsResponse {
            height: 42,
            fee_asset_ids: vec![
                asset::Id::from_str_unchecked("asset_0"),
                asset::Id::from_str_unchecked("asset_1"),
                asset::Id::from_str_unchecked("asset_2"),
            ],
        };
        let expected = raw::AllowedFeeAssetIdsResponse {
            height: 42,
            fee_asset_ids: vec![
                asset::Id::from_str_unchecked("asset_0")
                    .get()
                    .to_vec()
                    .into(),
                asset::Id::from_str_unchecked("asset_1")
                    .get()
                    .to_vec()
                    .into(),
                asset::Id::from_str_unchecked("asset_2")
                    .get()
                    .to_vec()
                    .into(),
            ],
        };
        let actual = native.into_raw();
        assert_eq!(expected, actual);
    }

    #[test]
    fn allowed_fee_asset_ids_roundtrip_is_correct() {
        let native = AllowedFeeAssetIdsResponse {
            height: 42,
            fee_asset_ids: vec![
                asset::Id::from_str_unchecked("asset_0"),
                asset::Id::from_str_unchecked("asset_1"),
                asset::Id::from_str_unchecked("asset_2"),
            ],
        };
        let expected = native.clone();
        let actual = native.into_raw().try_into_native().unwrap();
        assert_eq!(expected, actual);
    }
}
