pub mod v2 {
    use std::io::{
        self,
        Write,
    };

    use borsh::BorshSerialize;
    use pbjson_types::Timestamp;

    use crate::{
        generated::price_feed::oracle::v2 as raw,
        oracles::price_feed::types::v2::{
            CurrencyPair,
            CurrencyPairError,
            CurrencyPairId,
            CurrencyPairNonce,
            ParsePriceError,
            Price,
        },
        Protobuf,
    };

    #[derive(Debug, Clone, PartialEq)]
    pub struct QuotePrice {
        pub price: Price,
        pub block_timestamp: Timestamp,
        pub block_height: u64,
    }

    impl TryFrom<raw::QuotePrice> for QuotePrice {
        type Error = QuotePriceError;

        fn try_from(raw: raw::QuotePrice) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<QuotePrice> for raw::QuotePrice {
        fn from(quote_price: QuotePrice) -> Self {
            quote_price.into_raw()
        }
    }

    impl BorshSerialize for QuotePrice {
        fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            self.price.serialize(writer)?;
            self.block_timestamp.seconds.serialize(writer)?;
            self.block_timestamp.nanos.serialize(writer)?;
            self.block_height.serialize(writer)
        }
    }

    impl QuotePrice {
        /// Converts from a raw protobuf `QuotePrice` to a native `QuotePrice`.
        ///
        /// # Errors
        ///
        /// - if the `price` field is invalid
        /// - if the `block_timestamp` field is missing
        #[expect(
            clippy::needless_pass_by_value,
            reason = "more convenient signature as is"
        )]
        pub fn try_from_raw(raw: raw::QuotePrice) -> Result<Self, QuotePriceError> {
            let price = raw.price.parse().map_err(QuotePriceError::parse_price)?;
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
        fn parse_price(source: ParsePriceError) -> Self {
            Self(QuotePriceErrorKind::Price {
                source,
            })
        }

        #[must_use]
        fn missing_block_timestamp() -> Self {
            Self(QuotePriceErrorKind::MissingBlockTimestamp)
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("failed to validate wire type `{}`", raw::QuotePrice::full_name())]
    enum QuotePriceErrorKind {
        #[error("failed to parse `price` field")]
        Price { source: ParsePriceError },
        #[error("missing block timestamp")]
        MissingBlockTimestamp,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CurrencyPairState {
        pub price: Option<QuotePrice>,
        pub nonce: CurrencyPairNonce,
        pub id: CurrencyPairId,
    }

    impl TryFrom<raw::CurrencyPairState> for CurrencyPairState {
        type Error = CurrencyPairStateError;

        fn try_from(raw: raw::CurrencyPairState) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<CurrencyPairState> for raw::CurrencyPairState {
        fn from(currency_pair_state: CurrencyPairState) -> Self {
            currency_pair_state.into_raw()
        }
    }

    impl CurrencyPairState {
        /// Converts from a raw protobuf `CurrencyPairState` to a native `CurrencyPairState`.
        ///
        /// # Errors
        ///
        /// - if the `price` field is missing
        /// - if the `price` field is invalid
        pub fn try_from_raw(raw: raw::CurrencyPairState) -> Result<Self, CurrencyPairStateError> {
            let price = raw
                .price
                .map(QuotePrice::try_from_raw)
                .transpose()
                .map_err(CurrencyPairStateError::quote_price_parse_error)?;
            let nonce = CurrencyPairNonce::new(raw.nonce);
            let id = CurrencyPairId::new(raw.id);
            Ok(Self {
                price,
                nonce,
                id,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::CurrencyPairState {
            raw::CurrencyPairState {
                price: self.price.map(QuotePrice::into_raw),
                nonce: self.nonce.get(),
                id: self.id.get(),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairStateError(CurrencyPairStateErrorKind);

    impl CurrencyPairStateError {
        #[must_use]
        fn quote_price_parse_error(err: QuotePriceError) -> Self {
            Self(CurrencyPairStateErrorKind::QuotePriceParseError(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum CurrencyPairStateErrorKind {
        #[error("failed to parse quote price")]
        QuotePriceParseError(#[source] QuotePriceError),
    }

    #[derive(Debug, Clone, BorshSerialize)]
    pub struct CurrencyPairGenesis {
        pub currency_pair: CurrencyPair,
        pub currency_pair_price: Option<QuotePrice>,
        pub id: CurrencyPairId,
        pub nonce: CurrencyPairNonce,
    }

    impl TryFrom<raw::CurrencyPairGenesis> for CurrencyPairGenesis {
        type Error = CurrencyPairGenesisError;

        fn try_from(raw: raw::CurrencyPairGenesis) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<CurrencyPairGenesis> for raw::CurrencyPairGenesis {
        fn from(currency_pair_genesis: CurrencyPairGenesis) -> Self {
            currency_pair_genesis.into_raw()
        }
    }

    impl CurrencyPairGenesis {
        #[must_use]
        pub fn currency_pair(&self) -> &CurrencyPair {
            &self.currency_pair
        }

        #[must_use]
        pub fn currency_pair_price(&self) -> &Option<QuotePrice> {
            &self.currency_pair_price
        }

        #[must_use]
        pub fn id(&self) -> CurrencyPairId {
            self.id
        }

        #[must_use]
        pub fn nonce(&self) -> CurrencyPairNonce {
            self.nonce
        }

        /// Converts from a raw protobuf `raw::CurrencyPairGenesis` to a validated
        /// domain type [`CurrencyPairGenesis`].
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
            let currency_pair = raw
                .currency_pair
                .ok_or_else(|| CurrencyPairGenesisError::field_not_set("currency_pair"))?
                .try_into()
                .map_err(CurrencyPairGenesisError::currency_pair)?;
            let currency_pair_price = raw
                .currency_pair_price
                .map(QuotePrice::try_from_raw)
                .transpose()
                .map_err(CurrencyPairGenesisError::currency_pair_price)?;

            let id = CurrencyPairId::new(raw.id);
            let nonce = CurrencyPairNonce::new(raw.nonce);
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
                currency_pair_price: self.currency_pair_price.map(QuotePrice::into_raw),
                id: self.id.get(),
                nonce: self.nonce.get(),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct CurrencyPairGenesisError(#[from] CurrencyPairGenesisErrorKind);

    impl CurrencyPairGenesisError {
        #[must_use]
        fn field_not_set(name: &'static str) -> Self {
            CurrencyPairGenesisErrorKind::FieldNotSet {
                name,
            }
            .into()
        }

        fn currency_pair(source: CurrencyPairError) -> Self {
            CurrencyPairGenesisErrorKind::CurrencyPair {
                source,
            }
            .into()
        }

        #[must_use]
        fn currency_pair_price(err: QuotePriceError) -> Self {
            Self(CurrencyPairGenesisErrorKind::CurrencyPairPrice(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum CurrencyPairGenesisErrorKind {
        #[error("required field not set: .{name}")]
        FieldNotSet { name: &'static str },
        #[error("field `.currency_pair` was invalid")]
        CurrencyPair { source: CurrencyPairError },
        #[error("field `.currency_pair_price` was invalid")]
        CurrencyPairPrice(#[source] QuotePriceError),
    }

    #[derive(Debug, Clone, BorshSerialize)]
    pub struct GenesisState {
        pub currency_pair_genesis: Vec<CurrencyPairGenesis>,
        pub next_id: CurrencyPairId,
    }

    impl TryFrom<raw::GenesisState> for GenesisState {
        type Error = GenesisStateError;

        fn try_from(raw: raw::GenesisState) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<GenesisState> for raw::GenesisState {
        fn from(genesis_state: GenesisState) -> Self {
            genesis_state.into_raw()
        }
    }

    impl Protobuf for GenesisState {
        type Error = GenesisStateError;
        type Raw = raw::GenesisState;

        fn try_from_raw_ref(raw: &raw::GenesisState) -> Result<Self, GenesisStateError> {
            let currency_pair_genesis = raw
                .currency_pair_genesis
                .clone()
                .into_iter()
                .map(CurrencyPairGenesis::try_from_raw)
                .collect::<Result<Vec<_>, _>>()
                .map_err(GenesisStateError::currency_pair_genesis_parse_error)?;
            let next_id = CurrencyPairId::new(raw.next_id);
            Ok(Self {
                currency_pair_genesis,
                next_id,
            })
        }

        /// Converts from a raw protobuf `GenesisState` to a native `GenesisState`.
        ///
        /// # Errors
        ///
        /// - if any of the `currency_pair_genesis` are invalid
        fn try_from_raw(raw: raw::GenesisState) -> Result<Self, GenesisStateError> {
            let currency_pair_genesis = raw
                .currency_pair_genesis
                .into_iter()
                .map(CurrencyPairGenesis::try_from_raw)
                .collect::<Result<Vec<_>, _>>()
                .map_err(GenesisStateError::currency_pair_genesis_parse_error)?;
            let next_id = CurrencyPairId::new(raw.next_id);
            Ok(Self {
                currency_pair_genesis,
                next_id,
            })
        }

        fn to_raw(&self) -> raw::GenesisState {
            raw::GenesisState {
                currency_pair_genesis: self
                    .currency_pair_genesis
                    .clone()
                    .into_iter()
                    .map(CurrencyPairGenesis::into_raw)
                    .collect(),
                next_id: self.next_id.get(),
            }
        }

        #[must_use]
        fn into_raw(self) -> raw::GenesisState {
            raw::GenesisState {
                currency_pair_genesis: self
                    .currency_pair_genesis
                    .into_iter()
                    .map(CurrencyPairGenesis::into_raw)
                    .collect(),
                next_id: self.next_id.get(),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct GenesisStateError(GenesisStateErrorKind);

    impl GenesisStateError {
        #[must_use]
        fn currency_pair_genesis_parse_error(err: CurrencyPairGenesisError) -> Self {
            Self(GenesisStateErrorKind::CurrencyPairGenesisParseError(err))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum GenesisStateErrorKind {
        #[error("failed to parse genesis currency pair")]
        CurrencyPairGenesisParseError(#[source] CurrencyPairGenesisError),
    }
}
