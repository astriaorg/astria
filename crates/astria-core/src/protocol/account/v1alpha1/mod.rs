use super::raw;
use crate::primitive::v1::asset::{
    Denom,
    ParseDenomError,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AssetBalanceError(AssetBalanceErrorKind);
impl AssetBalanceError {
    #[must_use]
    fn invalid_denom(source: ParseDenomError) -> Self {
        Self(AssetBalanceErrorKind::InvalidDenom {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum AssetBalanceErrorKind {
    #[error("`denom` field was invalid")]
    InvalidDenom { source: ParseDenomError },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetBalance {
    pub denom: Denom,
    pub balance: u128,
}

impl AssetBalance {
    /// Converts a protobuf [`raw::AssetBalance`] to an astria
    /// native [`AssetBalance`].
    /// # Errors
    /// Returns an error if the protobuf `denom` field can't be pased as a [`Denom`].
    pub fn try_from_raw(proto: &raw::AssetBalance) -> Result<Self, AssetBalanceError> {
        let raw::AssetBalance {
            denom,
            balance,
        } = proto;
        Ok(Self {
            denom: denom.parse().map_err(AssetBalanceError::invalid_denom)?,
            balance: balance.map_or(0, Into::into),
        })
    }

    /// Converts an astria native [`AssetBalance`] to a
    /// protobuf [`raw::AssetBalance`].
    #[must_use]
    pub fn into_raw(self) -> raw::AssetBalance {
        raw::AssetBalance {
            denom: self.denom.to_string(),
            balance: Some(self.balance.into()),
        }
    }
}

impl raw::BalanceResponse {
    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`raw::BalanceResponse`].
    #[must_use]
    pub fn from_native(native: BalanceResponse) -> Self {
        let BalanceResponse {
            height,
            balances,
        } = native;
        Self {
            height,
            balances: balances.into_iter().map(AssetBalance::into_raw).collect(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BalanceResponseError(BalanceResponseErrorKind);

impl BalanceResponseError {
    #[must_use]
    fn asset_balance(source: AssetBalanceError) -> Self {
        Self(BalanceResponseErrorKind::AssetBalance {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum BalanceResponseErrorKind {
    #[error("`balances` contained an invalid asset balance")]
    AssetBalance { source: AssetBalanceError },
}

/// The sequencer response to a balance request for a given account at a given height.
#[derive(Clone, Debug, PartialEq)]
pub struct BalanceResponse {
    pub height: u64,
    pub balances: Vec<AssetBalance>,
}

impl BalanceResponse {
    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    ///
    /// # Errors
    /// Returns an error if one or more of the strings in the protobuf `balances` field can't
    /// be pased as a [`Denom`].
    pub fn try_from_raw(proto: &raw::BalanceResponse) -> Result<Self, BalanceResponseError> {
        let raw::BalanceResponse {
            height,
            balances,
        } = proto;
        Ok(Self {
            height: *height,
            balances: balances
                .iter()
                .map(AssetBalance::try_from_raw)
                .collect::<Result<_, _>>()
                .map_err(BalanceResponseError::asset_balance)?,
        })
    }

    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`raw::BalanceResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::BalanceResponse {
        raw::BalanceResponse::from_native(self)
    }
}

impl raw::NonceResponse {
    /// Converts a protobuf [`raw::NonceResponse`] to a native
    /// astria `NonceResponse`.
    #[must_use]
    pub fn from_native(native: NonceResponse) -> Self {
        let NonceResponse {
            height,
            nonce,
        } = native;
        Self {
            height,
            nonce,
        }
    }

    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    #[must_use]
    pub fn into_native(self) -> NonceResponse {
        NonceResponse::from_raw(&self)
    }

    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`] by allocating a new [`v1alpha::NonceResponse`].
    #[must_use]
    pub fn to_native(&self) -> NonceResponse {
        self.clone().into_native()
    }
}

/// The sequencer response to a nonce request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonceResponse {
    pub height: u64,
    pub nonce: u32,
}

impl NonceResponse {
    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    #[must_use]
    pub fn from_raw(proto: &raw::NonceResponse) -> Self {
        let raw::NonceResponse {
            height,
            nonce,
        } = *proto;
        Self {
            height,
            nonce,
        }
    }

    /// Converts an astria native [`NonceResponse`] to a
    /// protobuf [`raw::NonceResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::NonceResponse {
        raw::NonceResponse::from_native(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetBalance,
        BalanceResponse,
        NonceResponse,
    };

    #[test]
    fn balance_roundtrip_is_correct() {
        let balances = vec![AssetBalance {
            denom: "nria".parse().unwrap(),
            balance: 999,
        }];
        let expected = BalanceResponse {
            height: 42,
            balances,
        };
        let actual = BalanceResponse::try_from_raw(&expected.clone().into_raw()).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn nonce_roundtrip_is_correct() {
        let expected = NonceResponse {
            height: 42,
            nonce: 42,
        };
        let actual = expected.into_raw().into_native();
        assert_eq!(expected, actual);
    }
}
