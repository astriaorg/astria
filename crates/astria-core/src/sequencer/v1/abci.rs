use std::{
    borrow::Cow,
    num::NonZeroU32,
};

use super::raw;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct AbciErrorCode(u32);

#[rustfmt::skip]
impl AbciErrorCode {
    pub const UNSPECIFIED: Self = Self(0);
    pub const UNKNOWN_PATH: Self = Self(1);
    pub const INVALID_PARAMETER: Self = Self(2);
    pub const INTERNAL_ERROR: Self = Self(3);
    pub const INVALID_NONCE: Self = Self(4);
    pub const TRANSACTION_TOO_LARGE: Self = Self(5);
    pub const INSUFFICIENT_FUNDS: Self = Self(6);
}

impl AbciErrorCode {
    #[must_use]
    pub fn info(self) -> Cow<'static, str> {
        match self.0 {
            0 => "unspecified".into(),
            1 => "provided path is unknown".into(),
            2 => "one or more path parameters were invalid".into(),
            3 => "an internal server error occured".into(),
            4 => "the provided nonce was invalid".into(),
            5 => "the provided transaction was too large".into(),
            6 => "insufficient funds".into(),
            other => format!("unknown non-zero abci error code: {other}").into(),
        }
    }

    /// Converts from the rust representation of the abci error code.
    ///
    /// Note that by convention unknown protobuf enum variants are mapped to
    /// the default enum variant with value 0.
    #[must_use]
    pub fn from_raw(raw: raw::AbciErrorCode) -> Option<Self> {
        let code = match raw {
            raw::AbciErrorCode::Unspecified => Self::UNSPECIFIED,
            raw::AbciErrorCode::UnknownPath => Self::UNKNOWN_PATH,
            raw::AbciErrorCode::InvalidParameter => Self::INVALID_PARAMETER,
            raw::AbciErrorCode::InternalError => Self::INTERNAL_ERROR,
            raw::AbciErrorCode::InvalidNonce => Self::INVALID_NONCE,
            raw::AbciErrorCode::TransactionTooLarge => Self::TRANSACTION_TOO_LARGE,
        };
        Some(code)
    }
}

impl std::fmt::Display for AbciErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info())
    }
}

impl From<AbciErrorCode> for tendermint::abci::Code {
    fn from(value: AbciErrorCode) -> Self {
        value.0.into()
    }
}

impl From<NonZeroU32> for AbciErrorCode {
    fn from(value: NonZeroU32) -> Self {
        match value.get() {
            1 => Self::UNKNOWN_PATH,
            2 => Self::INVALID_PARAMETER,
            3 => Self::INTERNAL_ERROR,
            4 => Self::INVALID_NONCE,
            5 => Self::TRANSACTION_TOO_LARGE,
            other => Self(other),
        }
    }
}
