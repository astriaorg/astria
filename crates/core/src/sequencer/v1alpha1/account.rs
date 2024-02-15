use super::{
    asset::Denom,
    raw,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AssetBalance {
    pub denom: Denom,
    pub balance: u128,
}

impl AssetBalance {
    /// Converts a protobuf [`raw::AssetBalance`] to an astria
    /// native [`AssetBalance`].
    #[must_use]
    pub fn from_raw(proto: &raw::AssetBalance) -> Self {
        let raw::AssetBalance {
            denom,
            balance,
        } = proto;
        Self {
            denom: Denom::from(denom.as_str()),
            balance: balance.map_or(0, Into::into),
        }
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

    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    #[must_use]
    pub fn into_native(self) -> BalanceResponse {
        BalanceResponse::from_raw(&self)
    }

    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`] by allocating a new [`v1alpha::BalanceResponse`].
    #[must_use]
    pub fn to_native(&self) -> BalanceResponse {
        self.clone().into_native()
    }
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
    pub fn from_raw(proto: &raw::BalanceResponse) -> Self {
        let raw::BalanceResponse {
            height,
            balances,
        } = proto;
        Self {
            height: *height,
            balances: balances.iter().map(AssetBalance::from_raw).collect(),
        }
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
            denom: "nria".into(),
            balance: 999,
        }];
        let expected = BalanceResponse {
            height: 42,
            balances,
        };
        let actual = expected.clone().into_raw().into_native();
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
