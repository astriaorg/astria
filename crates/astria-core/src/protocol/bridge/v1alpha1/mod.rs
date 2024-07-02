use super::raw;
use crate::primitive::v1::{
    asset,
    asset::denom::ParseDenomError,
    Address,
    AddressError,
    IncorrectRollupIdLength,
    RollupId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeAccountLastTxHashResponse {
    pub height: u64,
    pub tx_hash: Option<[u8; 32]>,
}

impl BridgeAccountLastTxHashResponse {
    /// Converts a native [`BridgeAccountLastTxHashResponse`] to a protobuf
    /// [`raw::BridgeAccountLastTxHashResponse`].
    ///
    /// # Errors
    ///
    /// - if the transaction hash is not 32 bytes
    pub fn try_from_raw(
        raw: raw::BridgeAccountLastTxHashResponse,
    ) -> Result<Self, BridgeAccountLastTxHashResponseError> {
        Ok(Self {
            height: raw.height,
            tx_hash: raw
                .tx_hash
                .map(TryInto::<[u8; 32]>::try_into)
                .transpose()
                .map_err(|bytes: Vec<u8>| {
                    BridgeAccountLastTxHashResponseError::invalid_tx_hash(bytes.len())
                })?,
        })
    }

    #[must_use]
    pub fn into_raw(self) -> raw::BridgeAccountLastTxHashResponse {
        raw::BridgeAccountLastTxHashResponse {
            height: self.height,
            tx_hash: self.tx_hash.map(Into::into),
        }
    }
}

impl raw::BridgeAccountLastTxHashResponse {
    /// Converts a protobuf [`raw::BridgeAccountLastTxHashResponse`] to a native
    /// [`BridgeAccountLastTxHashResponse`].
    ///
    /// # Errors
    ///
    /// - if the transaction hash is not 32 bytes
    pub fn try_into_native(
        self,
    ) -> Result<BridgeAccountLastTxHashResponse, BridgeAccountLastTxHashResponseError> {
        BridgeAccountLastTxHashResponse::try_from_raw(self)
    }

    #[must_use]
    pub fn from_native(
        native: BridgeAccountLastTxHashResponse,
    ) -> raw::BridgeAccountLastTxHashResponse {
        native.into_raw()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeAccountLastTxHashResponseError(BridgeAccountLastTxHashResponseErrorKind);

impl BridgeAccountLastTxHashResponseError {
    #[must_use]
    pub fn invalid_tx_hash(bytes: usize) -> Self {
        Self(BridgeAccountLastTxHashResponseErrorKind::InvalidTxHash(
            bytes,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeAccountLastTxHashResponseErrorKind {
    #[error("invalid tx hash; must be 32 bytes, got {0} bytes")]
    InvalidTxHash(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeAccountInfoResponse {
    pub height: u64,
    pub info: Option<BridgeAccountInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeAccountInfo {
    pub rollup_id: RollupId,
    pub asset: asset::Denom,
    pub sudo_address: Address,
    pub withdrawer_address: Address,
}

impl BridgeAccountInfoResponse {
    /// Converts a protobuf [`raw::BridgeAccountInfoResponse`] to a native
    /// [`BridgeAccountInfoResponse`].
    ///
    /// # Errors
    ///
    /// - if the `rollup_id` field is set but the `sudo_address` field is not
    /// - if the `rollup_id` field is set but the `withdrawer_address` field is not
    /// - if the `rollup_id` field is set but the `asset_id` field is not
    /// - if the `asset` field does not contain a valid asset denom
    /// - if the `rollup_id` field is set but invalid
    /// - if the `sudo_address` field is set but invalid
    /// - if the `withdrawer_address` field is set but invalid
    pub fn try_from_raw(
        raw: raw::BridgeAccountInfoResponse,
    ) -> Result<Self, BridgeAccountInfoResponseError> {
        let raw::BridgeAccountInfoResponse {
            height,
            rollup_id,
            asset,
            sudo_address,
            withdrawer_address,
        } = raw;

        let Some(rollup_id) = rollup_id else {
            return Ok(Self {
                height,
                info: None,
            });
        };

        let Some(sudo_address) = sudo_address else {
            return Err(BridgeAccountInfoResponseError::field_not_set(
                "sudo_address",
            ));
        };

        let Some(withdrawer_address) = withdrawer_address else {
            return Err(BridgeAccountInfoResponseError::field_not_set(
                "withdrawer_address",
            ));
        };

        let Some(asset) = asset else {
            return Err(BridgeAccountInfoResponseError::field_not_set("asset"));
        };

        let asset = asset
            .parse()
            .map_err(BridgeAccountInfoResponseError::invalid_denom)?;

        Ok(Self {
            height,
            info: Some(BridgeAccountInfo {
                rollup_id: RollupId::try_from_raw(&rollup_id)
                    .map_err(BridgeAccountInfoResponseError::invalid_rollup_id)?,
                asset,
                sudo_address: Address::try_from_raw(&sudo_address)
                    .map_err(BridgeAccountInfoResponseError::invalid_sudo_address)?,
                withdrawer_address: Address::try_from_raw(&withdrawer_address)
                    .map_err(BridgeAccountInfoResponseError::invalid_withdrawer_address)?,
            }),
        })
    }

    #[must_use]
    pub fn into_raw(self) -> raw::BridgeAccountInfoResponse {
        let Some(info) = self.info else {
            return raw::BridgeAccountInfoResponse {
                height: self.height,
                rollup_id: None,
                asset: None,
                sudo_address: None,
                withdrawer_address: None,
            };
        };

        raw::BridgeAccountInfoResponse {
            height: self.height,
            rollup_id: Some(info.rollup_id.into_raw()),
            asset: Some(info.asset.to_string()),
            sudo_address: Some(info.sudo_address.into_raw()),
            withdrawer_address: Some(info.withdrawer_address.into_raw()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeAccountInfoResponseError(BridgeAccountInfoResponseErrorKind);

#[derive(Debug, thiserror::Error)]
enum BridgeAccountInfoResponseErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `denom` field was invalid")]
    InvalidDenom(#[source] ParseDenomError),
    #[error("the `rollup_id` field was invalid")]
    InvalidRollupId(#[source] IncorrectRollupIdLength),
    #[error("the `sudo_address` field was invalid")]
    InvalidSudoAddress(#[source] AddressError),
    #[error("the `withdrawer_address` field was invalid")]
    InvalidWithdrawerAddress(#[source] AddressError),
}

impl BridgeAccountInfoResponseError {
    #[must_use]
    pub fn field_not_set(field: &'static str) -> Self {
        Self(BridgeAccountInfoResponseErrorKind::FieldNotSet(field))
    }

    #[must_use]
    pub fn invalid_rollup_id(err: IncorrectRollupIdLength) -> Self {
        Self(BridgeAccountInfoResponseErrorKind::InvalidRollupId(err))
    }

    #[must_use]
    pub fn invalid_sudo_address(err: AddressError) -> Self {
        Self(BridgeAccountInfoResponseErrorKind::InvalidSudoAddress(err))
    }

    #[must_use]
    pub fn invalid_withdrawer_address(err: AddressError) -> Self {
        Self(BridgeAccountInfoResponseErrorKind::InvalidWithdrawerAddress(err))
    }

    #[must_use]
    pub fn invalid_denom(err: ParseDenomError) -> Self {
        Self(BridgeAccountInfoResponseErrorKind::InvalidDenom(err))
    }
}
