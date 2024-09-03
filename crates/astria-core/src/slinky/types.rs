pub mod v1 {
    use crate::generated::astria_vendored::slinky::types::v1 as raw;

    #[cfg_attr(
        feature = "serde",
        derive(serde::Deserialize, serde::Serialize),
        serde(from = "raw::CurrencyPair", into = "raw::CurrencyPair")
    )]
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CurrencyPair {
        base: String,
        quote: String,
    }

    impl CurrencyPair {
        #[must_use]
        pub fn base(&self) -> &str {
            &self.base
        }

        #[must_use]
        pub fn quote(&self) -> &str {
            &self.quote
        }

        #[must_use]
        pub fn from_raw(raw: raw::CurrencyPair) -> Self {
            Self {
                base: raw.base,
                quote: raw.quote,
            }
        }

        #[must_use]
        pub fn into_raw(self) -> raw::CurrencyPair {
            raw::CurrencyPair {
                base: self.base,
                quote: self.quote,
            }
        }
    }

    impl From<raw::CurrencyPair> for CurrencyPair {
        fn from(raw: raw::CurrencyPair) -> Self {
            Self::from_raw(raw)
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
            let re = regex::Regex::new(r"^([a-zA-Z]+)/([a-zA-Z]+)$").expect("valid regex");
            let caps = re
                .captures(s)
                .ok_or_else(|| CurrencyPairParseError::invalid_currency_pair_string(s))?;
            let base = caps
                .get(1)
                .expect("must have base string, as regex captured it")
                .as_str();
            let quote = caps
                .get(2)
                .expect("must have quote string, as regex captured it")
                .as_str();

            Ok(Self {
                base: base.to_string(),
                quote: quote.to_string(),
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
    }

    impl CurrencyPairParseError {
        #[must_use]
        pub fn invalid_currency_pair_string(s: &str) -> Self {
            Self(CurrencyPairParseErrorKind::InvalidCurrencyPairString(
                s.to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::v1::CurrencyPair;

    #[test]
    fn currency_pair_parse() {
        let currency_pair = "ETH/USD".parse::<CurrencyPair>().unwrap();
        assert_eq!(currency_pair.base(), "ETH");
        assert_eq!(currency_pair.quote(), "USD");
        assert_eq!(currency_pair.to_string(), "ETH/USD");
    }

    #[test]
    fn test_currency_pair_parse_invalid() {
        let currency_pair = "ETHUSD".parse::<CurrencyPair>();
        assert!(currency_pair.is_err());
    }
}
