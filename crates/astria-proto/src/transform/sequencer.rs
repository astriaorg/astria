use std::{
    error::Error,
    fmt::Display,
};

use crate::sequencer::v1alpha1;

impl v1alpha1::BalanceResponse {
    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`v1alpha1::BalanceResponse`].
    pub fn from_native(native: BalanceResponse) -> Self {
        let BalanceResponse {
            account,
            height,
            balance,
        } = native;
        Self {
            account: account.to_vec(),
            height,
            balance: Some(balance.into()),
        }
    }

    /// Converts a protobuf [`v1alpha1::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    pub fn into_native(self) -> Result<BalanceResponse, IncorrectAccountLength> {
        BalanceResponse::from_proto(self)
    }

    pub fn to_native(&self) -> Result<BalanceResponse, IncorrectAccountLength> {
        self.clone().into_native()
    }
}

/// The sequencer response to a balance request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceResponse {
    pub account: [u8; 20],
    pub height: u64,
    pub balance: u128,
}

impl BalanceResponse {
    /// Converts a protobuf [`v1alpha1::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    pub fn from_proto(proto: v1alpha1::BalanceResponse) -> Result<Self, IncorrectAccountLength> {
        let v1alpha1::BalanceResponse {
            account,
            height,
            balance,
        } = proto;
        Ok(Self {
            account: convert_bytes_to_account(account)?,
            height,
            balance: balance.map_or(0, Into::into),
        })
    }

    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`v1alpha1::BalanceResponse`].
    pub fn into_proto(self) -> v1alpha1::BalanceResponse {
        v1alpha1::BalanceResponse::from_native(self)
    }
}

impl v1alpha1::NonceResponse {
    /// Converts a protobuf [`v1alpha1::NonceResponse`] to a native
    /// astria `NonceResponse`.
    pub fn from_native(native: NonceResponse) -> Self {
        let NonceResponse {
            account,
            height,
            nonce,
        } = native;
        Self {
            account: account.to_vec(),
            height,
            nonce,
        }
    }

    /// Converts a protobuf [`v1alpha1::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    pub fn into_native(self) -> Result<NonceResponse, IncorrectAccountLength> {
        NonceResponse::from_proto(self)
    }

    pub fn to_native(&self) -> Result<NonceResponse, IncorrectAccountLength> {
        self.clone().into_native()
    }
}

/// The sequencer response to a nonce request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonceResponse {
    pub account: [u8; 20],
    pub height: u64,
    pub nonce: u32,
}

impl NonceResponse {
    /// Converts a protobuf [`v1alpha1::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    pub fn from_proto(proto: v1alpha1::NonceResponse) -> Result<Self, IncorrectAccountLength> {
        let v1alpha1::NonceResponse {
            account,
            height,
            nonce,
        } = proto;
        Ok(Self {
            account: convert_bytes_to_account(account)?,
            height,
            nonce,
        })
    }

    /// Converts an astria native [`NonceResponse`] to a
    /// protobuf [`v1alpha1::NonceResponse`].
    pub fn into_proto(self) -> v1alpha1::NonceResponse {
        v1alpha1::NonceResponse::from_native(self)
    }
}

/// Indicates that the protobuf response contained an array field that was not 20 bytes long.
#[derive(Debug)]
pub struct IncorrectAccountLength {
    received: usize,
}

impl Display for IncorrectAccountLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 20 bytes, got {}", self.received)
    }
}

impl Error for IncorrectAccountLength {}

fn convert_bytes_to_account(bytes: Vec<u8>) -> Result<[u8; 20], IncorrectAccountLength> {
    <[u8; 20]>::try_from(bytes).map_err(|bytes| IncorrectAccountLength {
        received: bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        convert_bytes_to_account,
        BalanceResponse,
    };
    use crate::transform::sequencer::{
        IncorrectAccountLength,
        NonceResponse,
    };

    #[test]
    fn balance_roundtrip_is_correct() {
        let expected = BalanceResponse {
            account: [42; 20],
            height: 42,
            balance: 42,
        };
        let actual = expected.into_proto().into_native().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn nonce_roundtrip_is_correct() {
        let expected = NonceResponse {
            account: [42; 20],
            height: 42,
            nonce: 42,
        };
        let actual = expected.into_proto().into_native().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn account_of_20_bytes_is_converted_correctly() {
        let expected = [42; 20];
        let account = expected.to_vec();
        let actual = convert_bytes_to_account(account).unwrap();
        assert_eq!(expected, actual);
    }

    #[track_caller]
    fn account_conversion_check(bad_account: Vec<u8>) {
        let error = convert_bytes_to_account(bad_account);
        assert!(
            matches!(error, Err(IncorrectAccountLength { .. })),
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
