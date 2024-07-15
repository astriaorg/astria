pub mod v1 {
    use pbjson_types::Timestamp;

    use crate::{
        generated::slinky::oracle::v1 as raw,
        slinky::types::v1::CurrencyPair,
    };

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Debug, Clone)]
    pub struct QuotePrice {
        pub price: u128,
        pub block_timestamp: Timestamp,
        pub block_height: u64,
    }

    impl QuotePrice {
        /// Converts from a raw protobuf `QuotePrice` to a native `QuotePrice`.
        ///
        /// # Errors
        ///
        /// - if the `price` field is invalid
        /// - if the `block_timestamp` field is missing
        pub fn try_from_raw(raw: raw::QuotePrice) -> Result<Self, QuotePriceError> {
            let price = raw
                .price
                .parse()
                .map_err(QuotePriceError::price_parse_error)?;
            let Some(block_timestamp) = raw.block_timestamp else {
                return Err(QuotePriceError::missing_block_timestamp());
            };
            let block_height = raw.block_height;
            Ok(Self {
                price,
                block_timestamp,
                block_height,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::QuotePrice {
            raw::QuotePrice {
                price: self.price.to_string(),
                block_timestamp: Some(self.block_timestamp),
                block_height: self.block_height,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct QuotePriceError(QuotePriceErrorKind);

    impl QuotePriceError {
        #[must_use]
        pub fn price_parse_error(err: std::num::ParseIntError) -> Self {
            Self(QuotePriceErrorKind::PriceParseError(err))
        }

        #[must_use]
        pub fn missing_block_timestamp() -> Self {
            Self(QuotePriceErrorKind::MissingBlockTimestamp)
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum QuotePriceErrorKind {
        #[error(transparent)]
        PriceParseError(#[from] std::num::ParseIntError),
        #[error("missing block timestamp")]
        MissingBlockTimestamp,
    }

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Debug, Clone)]
    pub struct CurrencyPairState {
        pub price: QuotePrice,
        pub nonce: u64,
        pub id: u64,
    }

    impl CurrencyPairState {
        /// Converts from a raw protobuf `CurrencyPairState` to a native `CurrencyPairState`.
        ///
        /// # Errors
        ///
        /// - if the `price` field is missing
        /// - if the `price` field is invalid
        pub fn try_from_raw(raw: raw::CurrencyPairState) -> Result<Self, CurrencyPairStateError> {
            let Some(price) = raw
                .price
                .map(QuotePrice::try_from_raw)
                .transpose()
                .map_err(CurrencyPairStateError::quote_price_parse_error)?
            else {
                return Err(CurrencyPairStateError::missing_price());
            };
            let nonce = raw.nonce;
            let id = raw.id;
            Ok(Self {
                price,
                nonce,
                id,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::CurrencyPairState {
            raw::CurrencyPairState {
                price: Some(self.price.into_raw()),
                nonce: self.nonce,
                id: self.id,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairStateError(CurrencyPairStateErrorKind);

    impl CurrencyPairStateError {
        #[must_use]
        pub fn missing_price() -> Self {
            Self(CurrencyPairStateErrorKind::MissingPrice)
        }

        #[must_use]
        pub fn quote_price_parse_error(err: QuotePriceError) -> Self {
            Self(CurrencyPairStateErrorKind::QuotePriceParseError(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum CurrencyPairStateErrorKind {
        #[error("missing price")]
        MissingPrice,
        #[error(transparent)]
        QuotePriceParseError(QuotePriceError),
    }

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Debug, Clone)]
    pub struct CurrencyPairGenesis {
        pub currency_pair: CurrencyPair,
        pub currency_pair_price: QuotePrice,
        pub id: u64,
        pub nonce: u64,
    }

    impl CurrencyPairGenesis {
        #[must_use]
        pub fn currency_pair(&self) -> &CurrencyPair {
            &self.currency_pair
        }

        #[must_use]
        pub fn currency_pair_price(&self) -> &QuotePrice {
            &self.currency_pair_price
        }

        #[must_use]
        pub fn id(&self) -> u64 {
            self.id
        }

        #[must_use]
        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        /// Converts from a raw protobuf `CurrencyPairGenesis` to a native
        /// `CurrencyPairGenesis`.
        ///
        /// # Errors
        ///
        /// - if the `currency_pair` field is missing
        /// - if the `currency_pair` field is invalid
        /// - if the `currency_pair_price` field is missing
        /// - if the `currency_pair_price` field is invalid
        pub fn try_from_raw(
            raw: raw::CurrencyPairGenesis,
        ) -> Result<Self, CurrencyPairGenesisError> {
            let Some(currency_pair) = raw.currency_pair.map(CurrencyPair::from_raw) else {
                return Err(CurrencyPairGenesisError::missing_currency_pair());
            };
            let Some(currency_pair_price) = raw
                .currency_pair_price
                .map(QuotePrice::try_from_raw)
                .transpose()
                .map_err(CurrencyPairGenesisError::quote_price_parse_error)?
            else {
                return Err(CurrencyPairGenesisError::missing_currency_pair_price());
            };
            let id = raw.id;
            let nonce = raw.nonce;
            Ok(Self {
                currency_pair,
                currency_pair_price,
                id,
                nonce,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::CurrencyPairGenesis {
            raw::CurrencyPairGenesis {
                currency_pair: Some(self.currency_pair.into_raw()),
                currency_pair_price: Some(self.currency_pair_price.into_raw()),
                id: self.id,
                nonce: self.nonce,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairGenesisError(CurrencyPairGenesisErrorKind);

    impl CurrencyPairGenesisError {
        #[must_use]
        pub fn missing_currency_pair() -> Self {
            Self(CurrencyPairGenesisErrorKind::MissingCurrencyPair)
        }

        #[must_use]
        pub fn missing_currency_pair_price() -> Self {
            Self(CurrencyPairGenesisErrorKind::MissingCurrencyPairPrice)
        }

        #[must_use]
        pub fn quote_price_parse_error(err: QuotePriceError) -> Self {
            Self(CurrencyPairGenesisErrorKind::QuotePriceParseError(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum CurrencyPairGenesisErrorKind {
        #[error("missing currency pair")]
        MissingCurrencyPair,
        #[error("missing currency pair price")]
        MissingCurrencyPairPrice,
        #[error(transparent)]
        QuotePriceParseError(QuotePriceError),
    }

    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Debug, Clone)]
    pub struct GenesisState {
        pub currency_pair_genesis: Vec<CurrencyPairGenesis>,
        pub next_id: u64,
    }

    impl GenesisState {
        /// Converts from a raw protobuf `GenesisState` to a native `GenesisState`.
        ///
        /// # Errors
        ///
        /// - if any of the `currency_pair_genesis` are invalid
        pub fn try_from_raw(raw: raw::GenesisState) -> Result<Self, GenesisStateError> {
            let currency_pair_genesis = raw
                .currency_pair_genesis
                .into_iter()
                .map(CurrencyPairGenesis::try_from_raw)
                .collect::<Result<Vec<_>, _>>()
                .map_err(GenesisStateError::currency_pair_genesis_parse_error)?;
            let next_id = raw.next_id;
            Ok(Self {
                currency_pair_genesis,
                next_id,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::GenesisState {
            raw::GenesisState {
                currency_pair_genesis: self
                    .currency_pair_genesis
                    .into_iter()
                    .map(CurrencyPairGenesis::into_raw)
                    .collect(),
                next_id: self.next_id,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct GenesisStateError(GenesisStateErrorKind);

    impl GenesisStateError {
        #[must_use]
        pub fn currency_pair_genesis_parse_error(err: CurrencyPairGenesisError) -> Self {
            Self(GenesisStateErrorKind::CurrencyPairGenesisParseError(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum GenesisStateErrorKind {
        #[error(transparent)]
        CurrencyPairGenesisParseError(CurrencyPairGenesisError),
    }
}
