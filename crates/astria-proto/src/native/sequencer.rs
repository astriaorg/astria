use std::{error::Error, fmt::Display};

use crate::generated::sequencer::v1alpha1;

pub const ADDRESS_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address(pub [u8; ADDRESS_LEN]);

impl Address {
    /// Construct a sequencer address from a [`ed25519_consensus::VerificationKey`].
    ///
    /// The first 20 bytes of the sha256 hash of the verification key is the address.
    #[must_use]
    pub fn from_verification_key(public_key: ed25519_consensus::VerificationKey) -> Self {
        use sha2::Digest as _;
        /// this ensures that `ADDRESS_LEN` is never accidentally changed to a value
        /// that would violate this assumption.
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(ADDRESS_LEN <= 32);
        let mut hasher = sha2::Sha256::new();
        hasher.update(public_key);
        let bytes: [u8; 32] = hasher.finalize().into();
        Self::try_from_slice(&bytes[..ADDRESS_LEN])
            .expect("can convert 32 byte hash to 20 byte array")
    }

    /// Convert a byte slice to an address.
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer was not 20 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectAddressLength> {
        let inner = <[u8; ADDRESS_LEN]>::try_from(bytes).map_err(|_| IncorrectAddressLength {
            received: bytes.len(),
        })?;
        Ok(Self::from_array(inner))
    }

    #[must_use]
    pub fn from_array(array: [u8; ADDRESS_LEN]) -> Self {
        Self(array)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; ADDRESS_LEN]> for Address {
    fn from(inner: [u8; ADDRESS_LEN]) -> Self {
        Self(inner)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use hex::ToHex as _;
        f.write_str(&self.encode_hex::<String>())
    }
}

impl v1alpha1::BalanceResponse {
    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`v1alpha1::BalanceResponse`].
    #[must_use]
    pub fn from_native(native: BalanceResponse) -> Self {
        let BalanceResponse {
            account,
            height,
            balance,
        } = native;
        Self {
            account: account.0.to_vec(),
            height,
            balance: Some(balance.into()),
        }
    }

    /// Converts a protobuf [`v1alpha1::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn into_native(self) -> Result<BalanceResponse, IncorrectAddressLength> {
        BalanceResponse::from_proto(self)
    }

    /// Converts a protobuf [`v1alpha1::BalanceResponse`] to an astria
    /// native [`BalanceResponse`] by allocating a new [`v1alpha::BalanceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn to_native(&self) -> Result<BalanceResponse, IncorrectAddressLength> {
        self.clone().into_native()
    }
}

/// The sequencer response to a balance request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceResponse {
    pub account: Address,
    pub height: u64,
    pub balance: u128,
}

impl BalanceResponse {
    /// Converts a protobuf [`v1alpha1::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn from_proto(proto: v1alpha1::BalanceResponse) -> Result<Self, IncorrectAddressLength> {
        let v1alpha1::BalanceResponse {
            account,
            height,
            balance,
        } = proto;
        Ok(Self {
            account: Address::try_from_slice(&account)?,
            height,
            balance: balance.map_or(0, Into::into),
        })
    }

    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`v1alpha1::BalanceResponse`].
    #[must_use]
    pub fn into_proto(self) -> v1alpha1::BalanceResponse {
        v1alpha1::BalanceResponse::from_native(self)
    }
}

impl v1alpha1::NonceResponse {
    /// Converts a protobuf [`v1alpha1::NonceResponse`] to a native
    /// astria `NonceResponse`.
    #[must_use]
    pub fn from_native(native: NonceResponse) -> Self {
        let NonceResponse {
            account,
            height,
            nonce,
        } = native;
        Self {
            account: account.0.to_vec(),
            height,
            nonce,
        }
    }

    /// Converts a protobuf [`v1alpha1::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn into_native(self) -> Result<NonceResponse, IncorrectAddressLength> {
        NonceResponse::from_proto(self)
    }

    /// Converts a protobuf [`v1alpha1::NonceResponse`] to an astria
    /// native [`NonceResponse`] by allocating a new [`v1alpha::NonceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn to_native(&self) -> Result<NonceResponse, IncorrectAddressLength> {
        self.clone().into_native()
    }
}

/// The sequencer response to a nonce request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonceResponse {
    pub account: Address,
    pub height: u64,
    pub nonce: u32,
}

impl NonceResponse {
    /// Converts a protobuf [`v1alpha1::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer could not be converted to an [`Address`] because
    /// it was not 20 bytes long.
    pub fn from_proto(proto: v1alpha1::NonceResponse) -> Result<Self, IncorrectAddressLength> {
        let v1alpha1::NonceResponse {
            account,
            height,
            nonce,
        } = proto;
        Ok(Self {
            account: Address::try_from_slice(&account)?,
            height,
            nonce,
        })
    }

    /// Converts an astria native [`NonceResponse`] to a
    /// protobuf [`v1alpha1::NonceResponse`].
    #[must_use]
    pub fn into_proto(self) -> v1alpha1::NonceResponse {
        v1alpha1::NonceResponse::from_native(self)
    }
}

/// Indicates that the protobuf response contained an array field that was not 20 bytes long.
#[derive(Debug)]
pub struct IncorrectAddressLength {
    received: usize,
}

impl Display for IncorrectAddressLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 20 bytes, got {}", self.received)
    }
}

impl Error for IncorrectAddressLength {}

#[cfg(test)]
mod tests {
    use super::{Address, BalanceResponse, IncorrectAddressLength, NonceResponse};

    #[test]
    fn balance_roundtrip_is_correct() {
        let expected = BalanceResponse {
            account: Address([42; 20]),
            height: 42,
            balance: 42,
        };
        let actual = expected.into_proto().into_native().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn nonce_roundtrip_is_correct() {
        let expected = NonceResponse {
            account: Address([42; 20]),
            height: 42,
            nonce: 42,
        };
        let actual = expected.into_proto().into_native().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn account_of_20_bytes_is_converted_correctly() {
        let expected = Address([42; 20]);
        let account_vec = expected.0.to_vec();
        let actual = Address::try_from_slice(&account_vec).unwrap();
        assert_eq!(expected, actual);
    }

    #[track_caller]
    fn account_conversion_check(bad_account: Vec<u8>) {
        let error = Address::try_from_slice(&*bad_account);
        assert!(
            matches!(error, Err(IncorrectAddressLength { .. })),
            "converting form incorrect sized account succeeded where it should have failed"
        );
    }

    #[test]
    fn account_of_incorrect_length_gives_error() {
        account_conversion_check(vec![42; 0]);
        account_conversion_check(vec![42; 19]);
        account_conversion_check(vec![42; 21]);
        account_conversion_check(vec![42; 100]);
    }
}
