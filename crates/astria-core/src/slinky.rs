pub mod abci {
    pub mod v1 {
        use std::collections::HashMap;

        use crate::generated::slinky::abci::v1 as raw;

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct OracleVoteExtension {
            pub prices: HashMap<u64, Vec<u8>>,
        }

        impl OracleVoteExtension {
            #[must_use]
            pub fn from_raw(raw: raw::OracleVoteExtension) -> Self {
                Self {
                    prices: raw.prices,
                }
            }

            #[must_use]
            pub fn into_raw(self) -> raw::OracleVoteExtension {
                raw::OracleVoteExtension {
                    prices: self.prices,
                }
            }
        }
    }
}

pub mod types {
    pub mod v1 {
        use crate::generated::slinky::types::v1 as raw;

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct CurrencyPair {
            base: String,
            quote: String,
        }

        impl CurrencyPair {
            #[must_use]
            pub fn new(base: String, quote: String) -> Self {
                Self {
                    base,
                    quote,
                }
            }

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

        impl std::fmt::Display for CurrencyPair {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}/{}", self.base, self.quote)
            }
        }

        impl std::str::FromStr for CurrencyPair {
            type Err = CurrencyPairParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() != 2 {
                    return Err(CurrencyPairParseError::invalid_currency_pair_string(s));
                }

                Ok(Self {
                    base: parts[0].to_string(),
                    quote: parts[1].to_string(),
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
            pub fn invalid_currency_pair_string(s: &str) -> Self {
                Self(CurrencyPairParseErrorKind::InvalidCurrencyPairString(
                    s.to_string(),
                ))
            }
        }
    }
}

pub mod market_map {
    pub mod v1 {
        use std::{
            collections::HashMap,
            str::FromStr,
        };

        use crate::{
            generated::slinky::marketmap::v1 as raw,
            primitive::v1::{
                Address,
                AddressError,
            },
            slinky::types::v1::CurrencyPair,
        };

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct GenesisState {
            pub market_map: MarketMap,
            pub last_updated: u64,
            pub params: Params,
        }

        impl GenesisState {
            pub fn try_from_raw(raw: raw::GenesisState) -> Result<Self, GenesisStateError> {
                let Some(market_map) = raw
                    .market_map
                    .map(MarketMap::try_from_raw)
                    .transpose()
                    .map_err(GenesisStateError::invalid_market_map)?
                else {
                    return Err(GenesisStateError::missing_market_map());
                };
                let last_updated = raw.last_updated;
                let Some(params) = raw
                    .params
                    .map(Params::try_from_raw)
                    .transpose()
                    .map_err(GenesisStateError::invalid_params)?
                else {
                    return Err(GenesisStateError::missing_params());
                };
                Ok(Self {
                    market_map,
                    last_updated,
                    params,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::GenesisState {
                raw::GenesisState {
                    market_map: Some(self.market_map.into_raw()),
                    last_updated: self.last_updated,
                    params: Some(self.params.into_raw()),
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct GenesisStateError(GenesisStateErrorKind);

        impl GenesisStateError {
            pub fn missing_market_map() -> Self {
                Self(GenesisStateErrorKind::MissingMarketMap)
            }

            pub fn invalid_market_map(err: MarketMapError) -> Self {
                Self(GenesisStateErrorKind::MarketMapParseError(err))
            }

            pub fn missing_params() -> Self {
                Self(GenesisStateErrorKind::MissingParams)
            }

            pub fn invalid_params(err: ParamsError) -> Self {
                Self(GenesisStateErrorKind::ParamsParseError(err))
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum GenesisStateErrorKind {
            #[error("missing market map")]
            MissingMarketMap,
            #[error(transparent)]
            MarketMapParseError(#[from] MarketMapError),
            #[error("missing params")]
            MissingParams,
            #[error(transparent)]
            ParamsParseError(#[from] ParamsError),
        }

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Params {
            pub market_authorities: Vec<Address>,
            pub admin: Address,
        }

        impl Params {
            pub fn try_from_raw(raw: raw::Params) -> Result<Self, ParamsError> {
                let market_authorities = raw
                    .market_authorities
                    .into_iter()
                    .map(|s| Address::from_str(&s))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ParamsError::market_authority_parse_error)?;
                let admin = raw.admin.parse().map_err(ParamsError::admin_parse_error)?;
                Ok(Self {
                    market_authorities,
                    admin,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::Params {
                raw::Params {
                    market_authorities: self
                        .market_authorities
                        .into_iter()
                        .map(|a| a.to_string())
                        .collect(),
                    admin: self.admin.to_string(),
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct ParamsError(ParamsErrorKind);

        impl ParamsError {
            pub fn market_authority_parse_error(err: AddressError) -> Self {
                Self(ParamsErrorKind::MarketAuthorityParseError(err))
            }

            pub fn admin_parse_error(err: AddressError) -> Self {
                Self(ParamsErrorKind::AdminParseError(err))
            }
        }

        #[derive(Debug, thiserror::Error)]
        pub enum ParamsErrorKind {
            #[error("failed to parse market authority address")]
            MarketAuthorityParseError(#[source] AddressError),
            #[error("failed to parse admin address")]
            AdminParseError(#[source] AddressError),
        }

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Market {
            ticker: Ticker,
            provider_configs: Vec<ProviderConfig>,
        }

        impl Market {
            pub fn try_from_raw(raw: raw::Market) -> Result<Self, MarketError> {
                let Some(ticker) = raw
                    .ticker
                    .map(Ticker::try_from_raw)
                    .transpose()
                    .map_err(MarketError::invalid_ticker)?
                else {
                    return Err(MarketError::missing_ticker());
                };

                let provider_configs = raw
                    .provider_configs
                    .into_iter()
                    .map(ProviderConfig::try_from_raw)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(MarketError::invalid_provider_config)?;
                Ok(Self {
                    ticker,
                    provider_configs,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::Market {
                raw::Market {
                    ticker: Some(self.ticker.into_raw()),
                    provider_configs: self
                        .provider_configs
                        .into_iter()
                        .map(ProviderConfig::into_raw)
                        .collect(),
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct MarketError(MarketErrorKind);

        impl MarketError {
            pub fn missing_ticker() -> Self {
                Self(MarketErrorKind::MissingTicker)
            }

            pub fn invalid_ticker(err: TickerError) -> Self {
                Self(MarketErrorKind::TickerParseError(err))
            }

            pub fn invalid_provider_config(err: ProviderConfigError) -> Self {
                Self(MarketErrorKind::ProviderConfigParseError(err))
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum MarketErrorKind {
            #[error("missing ticker")]
            MissingTicker,
            #[error(transparent)]
            TickerParseError(#[from] TickerError),
            #[error(transparent)]
            ProviderConfigParseError(#[from] ProviderConfigError),
        }

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Ticker {
            currency_pair: CurrencyPair,
            decimals: u64,
            min_provider_count: u64,
            enabled: bool,
            metadata_json: String,
        }

        impl Ticker {
            pub fn try_from_raw(raw: raw::Ticker) -> Result<Self, TickerError> {
                let Some(currency_pair) = raw.currency_pair.map(CurrencyPair::from_raw) else {
                    return Err(TickerError::missing_currency_pair());
                };
                Ok(Self {
                    currency_pair,
                    decimals: raw.decimals,
                    min_provider_count: raw.min_provider_count,
                    enabled: raw.enabled,
                    metadata_json: raw.metadata_json,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::Ticker {
                raw::Ticker {
                    currency_pair: Some(self.currency_pair.into_raw()),
                    decimals: self.decimals,
                    min_provider_count: self.min_provider_count,
                    enabled: self.enabled,
                    metadata_json: self.metadata_json,
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct TickerError(TickerErrorKind);

        impl TickerError {
            pub fn missing_currency_pair() -> Self {
                Self(TickerErrorKind::MissingCurrencyPair)
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum TickerErrorKind {
            #[error("missing currency pair")]
            MissingCurrencyPair,
        }

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct ProviderConfig {
            name: String,
            off_chain_ticker: String,
            normalize_by_pair: CurrencyPair,
            invert: bool,
            metadata_json: String,
        }

        impl ProviderConfig {
            pub fn try_from_raw(raw: raw::ProviderConfig) -> Result<Self, ProviderConfigError> {
                let Some(normalize_by_pair) = raw.normalize_by_pair.map(CurrencyPair::from_raw)
                else {
                    return Err(ProviderConfigError::missing_normalize_by_pair());
                };
                Ok(Self {
                    name: raw.name,
                    off_chain_ticker: raw.off_chain_ticker,
                    normalize_by_pair,
                    invert: raw.invert,
                    metadata_json: raw.metadata_json,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::ProviderConfig {
                raw::ProviderConfig {
                    name: self.name,
                    off_chain_ticker: self.off_chain_ticker,
                    normalize_by_pair: Some(self.normalize_by_pair.into_raw()),
                    invert: self.invert,
                    metadata_json: self.metadata_json,
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct ProviderConfigError(ProviderConfigErrorKind);

        impl ProviderConfigError {
            pub fn missing_normalize_by_pair() -> Self {
                Self(ProviderConfigErrorKind::MissingNormalizeByPair)
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum ProviderConfigErrorKind {
            #[error("missing normalize by pair")]
            MissingNormalizeByPair,
        }

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct MarketMap {
            pub markets: HashMap<String, Market>,
        }

        impl MarketMap {
            pub fn try_from_raw(raw: raw::MarketMap) -> Result<Self, MarketMapError> {
                let mut markets = HashMap::new();
                for (k, v) in raw.markets {
                    let market = Market::try_from_raw(v)
                        .map_err(|e| MarketMapError::invalid_market(k.clone(), e))?;
                    markets.insert(k, market);
                }
                Ok(Self {
                    markets,
                })
            }

            #[must_use]
            pub fn into_raw(self) -> raw::MarketMap {
                let markets = self
                    .markets
                    .into_iter()
                    .map(|(k, v)| (k, v.into_raw()))
                    .collect();
                raw::MarketMap {
                    markets,
                }
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error(transparent)]
        pub struct MarketMapError(MarketMapErrorKind);

        impl MarketMapError {
            pub fn invalid_market(name: String, err: MarketError) -> Self {
                Self(MarketMapErrorKind::InvalidMarket(name, err))
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum MarketMapErrorKind {
            #[error("invalid market {0}")]
            InvalidMarket(String, MarketError),
        }
    }
}

pub mod oracle {
    pub mod v1 {
        use pbjson_types::Timestamp;

        use crate::{
            generated::slinky::oracle::v1 as raw,
            slinky::types::v1::CurrencyPair,
        };

        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[derive(Debug, Clone)]
        pub struct QuotePrice {
            price: u128,
            block_timestamp: Timestamp,
            block_height: u64,
        }

        impl QuotePrice {
            pub fn try_from_raw(raw: raw::QuotePrice) -> Result<Self, QuotePriceError> {
                let price = raw
                    .price
                    .parse()
                    .map_err(QuotePriceError::price_parse_error)?;
                let Some(block_timestamp) = raw.block_timestamp.clone() else {
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
            pub fn price_parse_error(err: std::num::ParseIntError) -> Self {
                Self(QuotePriceErrorKind::PriceParseError(err))
            }

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
            pub fn try_from_raw(
                raw: raw::CurrencyPairState,
            ) -> Result<Self, CurrencyPairStateError> {
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
            pub fn missing_price() -> Self {
                Self(CurrencyPairStateErrorKind::MissingPrice)
            }

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
            currency_pair: CurrencyPair,
            currency_pair_price: QuotePrice,
            id: u64,
            nonce: u64,
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
            pub fn missing_currency_pair() -> Self {
                Self(CurrencyPairGenesisErrorKind::MissingCurrencyPair)
            }

            pub fn missing_currency_pair_price() -> Self {
                Self(CurrencyPairGenesisErrorKind::MissingCurrencyPairPrice)
            }

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
}
