use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "we want consistent and specific naming"
)]
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
    pub const TRANSACTION_FAILED_EXECUTION: Self = Self(unsafe { NonZeroU32::new_unchecked(10) });
    pub const TRANSACTION_INSERTION_FAILED: Self = Self(unsafe { NonZeroU32::new_unchecked(11) });
    pub const LOWER_NONCE_INVALIDATED: Self = Self(unsafe { NonZeroU32::new_unchecked(12) });
    pub const BAD_REQUEST: Self = Self(unsafe { NonZeroU32::new_unchecked(13) });
    pub const ALREADY_PRESENT: Self = Self(unsafe { NonZeroU32::new_unchecked(14) });
    pub const NONCE_TAKEN: Self = Self(unsafe { NonZeroU32::new_unchecked(15) });
    pub const ACCOUNT_SIZE_LIMIT: Self = Self(unsafe { NonZeroU32::new_unchecked(16) });
    pub const PARKED_FULL: Self = Self(unsafe { NonZeroU32::new_unchecked(17) });
    pub const TRANSACTION_INCLUDED_IN_BLOCK: Self = Self(unsafe { NonZeroU32::new_unchecked(18) });
    pub const TRANSACTION_FAILED_CHECK_TX: Self = Self(unsafe { NonZeroU32::new_unchecked(19) });
    pub const INVALID_TRANSACTION_BYTES: Self = Self(unsafe { NonZeroU32::new_unchecked(20) });
    pub const INVALID_TRANSACTION: Self = Self(unsafe { NonZeroU32::new_unchecked(21) });
    // NOTE: When adding a new code, ensure it is added to `ALL_CODES` in the `tests` module below.
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
            Self::TRANSACTION_FAILED_EXECUTION => {
                "the transaction failed to execute in prepare_proposal()".into()
            }
            Self::TRANSACTION_INSERTION_FAILED => {
                "the transaction failed insertion into the mempool".into()
            }
            Self::LOWER_NONCE_INVALIDATED => "lower nonce was invalidated in mempool".into(),
            Self::BAD_REQUEST => "the request payload was malformed".into(),
            Self::ALREADY_PRESENT => "the transaction is already present in the mempool".into(),
            Self::NONCE_TAKEN => "there is already a transaction with the same nonce for the \
                                  account in the mempool"
                .into(),
            Self::ACCOUNT_SIZE_LIMIT => {
                "the account has reached the maximum number of parked transactions".into()
            }
            Self::PARKED_FULL => "the mempool is out of space for more parked transactions".into(),
            Self::TRANSACTION_INCLUDED_IN_BLOCK => "the transaction was removed from the mempool \
                                                    after being included in a block"
                .into(),
            Self::TRANSACTION_FAILED_CHECK_TX => "the transaction failed check_tx".into(),
            Self::INVALID_TRANSACTION_BYTES => "the provided transaction bytes were invalid".into(),
            Self::INVALID_TRANSACTION => "the provided transaction was invalid".into(),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const ALL_CODES: [AbciErrorCode; 21] = [
        AbciErrorCode::UNKNOWN_PATH,
        AbciErrorCode::INVALID_PARAMETER,
        AbciErrorCode::INTERNAL_ERROR,
        AbciErrorCode::INVALID_NONCE,
        AbciErrorCode::TRANSACTION_TOO_LARGE,
        AbciErrorCode::INSUFFICIENT_FUNDS,
        AbciErrorCode::INVALID_CHAIN_ID,
        AbciErrorCode::VALUE_NOT_FOUND,
        AbciErrorCode::TRANSACTION_EXPIRED,
        AbciErrorCode::TRANSACTION_FAILED_EXECUTION,
        AbciErrorCode::TRANSACTION_INSERTION_FAILED,
        AbciErrorCode::LOWER_NONCE_INVALIDATED,
        AbciErrorCode::BAD_REQUEST,
        AbciErrorCode::ALREADY_PRESENT,
        AbciErrorCode::NONCE_TAKEN,
        AbciErrorCode::ACCOUNT_SIZE_LIMIT,
        AbciErrorCode::PARKED_FULL,
        AbciErrorCode::TRANSACTION_INCLUDED_IN_BLOCK,
        AbciErrorCode::TRANSACTION_FAILED_CHECK_TX,
        AbciErrorCode::INVALID_TRANSACTION_BYTES,
        AbciErrorCode::INVALID_TRANSACTION,
    ];

    #[test]
    fn error_code_snapshots() {
        for error_code in ALL_CODES {
            let name = format!("AbciErrorCode::{}", error_code.value());
            insta::assert_snapshot!(name, error_code);
        }
    }

    #[test]
    fn ensure_codes_are_unique() {
        let mut values = BTreeSet::new();
        for code in ALL_CODES {
            assert!(values.insert(code.value()), "duplicate value for {code:?}");
        }
    }
}
