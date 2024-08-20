use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct AbciErrorCode(NonZeroU32);

#[rustfmt::skip]
impl AbciErrorCode {
    pub const UNKNOWN_PATH: Self = Self(unsafe { NonZeroU32::new_unchecked(1) });
    pub const INVALID_PARAMETER: Self = Self(unsafe { NonZeroU32::new_unchecked(2) });
    pub const INTERNAL_ERROR: Self = Self(unsafe { NonZeroU32::new_unchecked(3) });
    pub const INVALID_NONCE: Self = Self(unsafe { NonZeroU32::new_unchecked(4) });
    pub const TRANSACTION_TOO_LARGE: Self = Self(unsafe { NonZeroU32::new_unchecked(5) });
    pub const INSUFFICIENT_FUNDS: Self = Self(unsafe { NonZeroU32::new_unchecked(6) });
    pub const INVALID_CHAIN_ID: Self = Self(unsafe { NonZeroU32::new_unchecked(7) });
    pub const VALUE_NOT_FOUND: Self = Self(unsafe { NonZeroU32::new_unchecked(8) });
    pub const TRANSACTION_EXPIRED: Self = Self(unsafe { NonZeroU32::new_unchecked(9) });
    pub const TRANSACTION_FAILED: Self = Self(unsafe { NonZeroU32::new_unchecked(10) });
    pub const BAD_REQUEST: Self = Self(unsafe { NonZeroU32::new_unchecked(11) });
}

impl AbciErrorCode {
    /// Returns the wrapped `NonZeroU32`.
    #[must_use]
    pub const fn value(self) -> NonZeroU32 {
        self.0
    }

    /// Returns brief information on the meaning of the error.
    #[must_use]
    pub fn info(self) -> String {
        match self {
            Self::UNKNOWN_PATH => "provided path is unknown".into(),
            Self::INVALID_PARAMETER => "one or more path parameters were invalid".into(),
            Self::INTERNAL_ERROR => "an internal server error occurred".into(),
            Self::INVALID_NONCE => "the provided nonce was invalid".into(),
            Self::TRANSACTION_TOO_LARGE => "the provided transaction was too large".into(),
            Self::INSUFFICIENT_FUNDS => "insufficient funds".into(),
            Self::INVALID_CHAIN_ID => "the provided chain id was invalid".into(),
            Self::VALUE_NOT_FOUND => "the requested value was not found".into(),
            Self::TRANSACTION_EXPIRED => "the transaction expired in the app's mempool".into(),
            Self::TRANSACTION_FAILED => {
                "the transaction failed to execute in prepare_proposal()".into()
            }
            Self::BAD_REQUEST => "the request payload was malformed".into(),
            Self(other) => {
                format!("invalid error code {other}: should be unreachable (this is a bug)")
            }
        }
    }
}

impl std::fmt::Display for AbciErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.0, self.info())
    }
}
