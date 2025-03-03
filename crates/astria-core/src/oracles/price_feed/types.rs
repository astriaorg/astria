pub mod v2 {
    use std::{
        fmt::Display,
        num::ParseIntError,
        str::FromStr,
    };

    use base64::{
        display::Base64Display,
        prelude::BASE64_STANDARD,
    };
    use borsh::BorshSerialize;
    use bytes::Bytes;

    use crate::generated::price_feed::types::v2 as raw;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, BorshSerialize)]
    pub struct Price(i128);

    impl Price {
        #[must_use]
        pub const fn new(value: i128) -> Self {
            Self(value)
        }

        #[must_use]
        pub fn get(self) -> i128 {
            self.0
        }
    }

    impl Price {
        pub fn checked_add(self, rhs: Self) -> Option<Self> {
            self.get().checked_add(rhs.get()).map(Self)
        }

        pub fn checked_div(self, rhs: i128) -> Option<Self> {
            self.get().checked_div(rhs).map(Self)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct ParsePriceError(#[from] ParseIntError);

    impl FromStr for Price {
        type Err = ParsePriceError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            s.parse().map(Self::new).map_err(Into::into)
        }
    }

    impl Display for Price {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(
        "failed decoding `{}` as u128 integer",
        Base64Display::new(.input, &BASE64_STANDARD)
    )]
    pub struct DecodePriceError {
        input: Bytes,
    }

    impl TryFrom<Bytes> for Price {
        type Error = DecodePriceError;

        fn try_from(input: Bytes) -> Result<Self, Self::Error> {
            // throw away the error because it does not contain extra information.
            let be_bytes = <[u8; 16]>::try_from(&*input).map_err(|_| Self::Error {
                input,
            })?;
            Ok(Price::new(i128::from_be_bytes(be_bytes)))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize)]
    pub struct Base(String);

    impl Base {
        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(value: String) -> Self {
            Self(value)
        }
    }

    impl Display for Base {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(
        "failed to parse input `{input}` as base part of currency pair; only ascii alpha \
         characters are permitted"
    )]
    pub struct ParseBaseError {
        input: String,
    }

    impl FromStr for Base {
        type Err = ParseBaseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            static REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            fn get_regex() -> &'static regex::Regex {
                REGEX.get_or_init(|| regex::Regex::new(r"^[a-zA-Z]+$").expect("valid regex"))
            }
            // allocating here because the string will always be allocated on both branches.
            // TODO: check if this string can be represented by a stack-optimized alternative
            //       like ecow, compact_str, or similar.
            let input = s.to_string();
            if get_regex().find(s).is_none() {
                return Err(Self::Err {
                    input,
                });
            }
            Ok(Self(input))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize)]
    pub struct Quote(String);

    impl Quote {
        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(value: String) -> Self {
            Self(value)
        }
    }

    impl Display for Quote {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(
        "failed to parse input `{input}` as quote part of currency pair; only ascii alpha \
         characters are permitted"
    )]
    pub struct ParseQuoteError {
        input: String,
    }

    impl FromStr for Quote {
        type Err = ParseQuoteError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            static REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            fn get_regex() -> &'static regex::Regex {
                REGEX.get_or_init(|| regex::Regex::new(r"^[a-zA-Z]+$").expect("valid regex"))
            }
            // allocating here because the string will always be allocated on both branches.
            // TODO: check if this string can be represented by a stack-optimized alternative
            //       like ecow, compact_str, or similar.
            let input = s.to_string();
            if get_regex().find(s).is_none() {
                return Err(Self::Err {
                    input,
                });
            }
            Ok(Self(input))
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairError(#[from] CurrencyPairErrorKind);

    #[derive(Debug, thiserror::Error)]
    #[error("failed validating wire type `{}`", CurrencyPair::full_name())]
    enum CurrencyPairErrorKind {
        #[error("invalid field `.base`")]
        ParseBase { source: ParseBaseError },
        #[error("invalid field `.quote`")]
        ParseQuote { source: ParseQuoteError },
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize)]
    pub struct CurrencyPair {
        base: Base,
        quote: Quote,
    }

    impl CurrencyPair {
        #[must_use]
        pub fn from_parts(base: Base, quote: Quote) -> Self {
            Self {
                base,
                quote,
            }
        }

        /// Returns the `(base, quote)` pair that makes up this [`CurrencyPair`].
        #[must_use]
        pub fn into_parts(self) -> (String, String) {
            (self.base.0, self.quote.0)
        }

        #[must_use]
        pub fn base(&self) -> &str {
            &self.base.0
        }

        #[must_use]
        pub fn quote(&self) -> &str {
            &self.quote.0
        }

        /// Converts a on-wire [`raw::CurrencyPair`] to a validated domain type [`CurrencyPair`].
        ///
        /// # Errors
        ///
        /// Returns an error if:
        /// - The `.base` field could not be parsed as a [`Base`].
        /// - The `.quote` field could not be parsed as a [`Quote`].
        #[expect(clippy::needless_pass_by_value, reason = "symmetry with other types")]
        pub fn try_from_raw(raw: raw::CurrencyPair) -> Result<Self, CurrencyPairError> {
            let base = raw
                .base
                .parse()
                .map_err(|source| CurrencyPairErrorKind::ParseBase {
                    source,
                })?;
            let quote = raw
                .quote
                .parse()
                .map_err(|source| CurrencyPairErrorKind::ParseQuote {
                    source,
                })?;
            Ok(Self {
                base,
                quote,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::CurrencyPair {
            raw::CurrencyPair {
                base: self.base.0,
                quote: self.quote.0,
            }
        }

        #[must_use]
        pub fn to_raw(&self) -> raw::CurrencyPair {
            raw::CurrencyPair {
                base: self.base.0.clone(),
                quote: self.quote.0.clone(),
            }
        }
    }

    impl TryFrom<raw::CurrencyPair> for CurrencyPair {
        type Error = CurrencyPairError;

        fn try_from(raw: raw::CurrencyPair) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<CurrencyPair> for raw::CurrencyPair {
        fn from(currency_pair: CurrencyPair) -> Self {
            currency_pair.into_raw()
        }
    }

    impl std::fmt::Display for CurrencyPair {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}/{}", self.base, self.quote)
        }
    }

    impl std::str::FromStr for CurrencyPair {
        type Err = CurrencyPairParseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let Some((base_str, quote_str)) = s.split_once('/') else {
                return Err(CurrencyPairParseError::invalid_currency_pair_string(s));
            };

            let base = base_str
                .parse()
                .map_err(CurrencyPairParseError::parse_base)?;
            let quote = quote_str
                .parse()
                .map_err(CurrencyPairParseError::parse_quote)?;

            Ok(Self {
                base,
                quote,
            })
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairParseError(CurrencyPairParseErrorKind);

    #[derive(Debug, thiserror::Error)]
    pub enum CurrencyPairParseErrorKind {
        #[error("invalid currency pair string: {0}")]
        InvalidCurrencyPairString(String),
        #[error(transparent)]
        ParseBase { source: ParseBaseError },
        #[error(transparent)]
        ParseQuote { source: ParseQuoteError },
    }

    impl CurrencyPairParseError {
        #[must_use]
        fn invalid_currency_pair_string(s: &str) -> Self {
            Self(CurrencyPairParseErrorKind::InvalidCurrencyPairString(
                s.to_string(),
            ))
        }

        #[must_use]
        fn parse_base(source: ParseBaseError) -> Self {
            Self(CurrencyPairParseErrorKind::ParseBase {
                source,
            })
        }

        #[must_use]
        fn parse_quote(source: ParseQuoteError) -> Self {
            Self(CurrencyPairParseErrorKind::ParseQuote {
                source,
            })
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize)]
    pub struct CurrencyPairId(u64);

    impl std::fmt::Display for CurrencyPairId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl CurrencyPairId {
        #[must_use]
        pub const fn new(value: u64) -> Self {
            Self(value)
        }

        #[must_use]
        pub fn get(self) -> u64 {
            self.0
        }

        #[must_use]
        pub fn increment(self) -> Option<Self> {
            let new_id = self.get().checked_add(1)?;
            Some(Self::new(new_id))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize)]
    pub struct CurrencyPairNonce(u64);

    impl std::fmt::Display for CurrencyPairNonce {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl CurrencyPairNonce {
        #[must_use]
        pub const fn new(value: u64) -> Self {
            Self(value)
        }

        #[must_use]
        pub fn get(self) -> u64 {
            self.0
        }

        #[must_use]
        pub fn increment(self) -> Option<Self> {
            let new_nonce = self.get().checked_add(1)?;
            Some(Self::new(new_nonce))
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn base_should_parse() {
            "ETH".parse::<Base>().unwrap();
        }

        #[test]
        fn invalid_base_should_not_parse() {
            " ETH".parse::<Base>().unwrap_err();
            "ETH ".parse::<Base>().unwrap_err();
            "ET H".parse::<Base>().unwrap_err();
        }

        #[test]
        fn quote_should_parse() {
            "ETH".parse::<Quote>().unwrap();
        }

        #[test]
        fn invalid_quote_should_not_parse() {
            " ETH".parse::<Quote>().unwrap_err();
            "ETH ".parse::<Quote>().unwrap_err();
            "ET H".parse::<Quote>().unwrap_err();
        }

        #[test]
        fn currency_pair_should_parse() {
            let currency_pair = "ETH/USD".parse::<CurrencyPair>().unwrap();
            assert_eq!(currency_pair.base(), "ETH");
            assert_eq!(currency_pair.quote(), "USD");
            assert_eq!(currency_pair.to_string(), "ETH/USD");
        }

        #[test]
        fn invalid_currency_pair_should_not_parse() {
            "ETHUSD".parse::<CurrencyPair>().unwrap_err();
            " ETH/USD".parse::<CurrencyPair>().unwrap_err();
            "ETH/USD ".parse::<CurrencyPair>().unwrap_err();
            "ETH /USD".parse::<CurrencyPair>().unwrap_err();
            "ETH/ USD".parse::<CurrencyPair>().unwrap_err();
        }

        #[test]
        fn can_parse_negative_price() {
            let price = "-1".parse::<Price>().unwrap();
            assert_eq!(price, Price::new(-1));
        }
    }
}
