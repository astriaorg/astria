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
        /// Converts from a raw protobuf `GenesisState` to a native `GenesisState`.
        ///
        /// # Errors
        ///
        /// - if the `market_map` field is missing
        /// - if the `market_map` field is invalid
        /// - if the `params` field is missing
        /// - if the `params` field is invalid
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
        #[must_use]
        pub fn missing_market_map() -> Self {
            Self(GenesisStateErrorKind::MissingMarketMap)
        }

        #[must_use]
        pub fn invalid_market_map(err: MarketMapError) -> Self {
            Self(GenesisStateErrorKind::MarketMapParseError(err))
        }

        #[must_use]
        pub fn missing_params() -> Self {
            Self(GenesisStateErrorKind::MissingParams)
        }

        #[must_use]
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
        /// Converts from a raw protobuf `Params` to a native `Params`.
        ///
        /// # Errors
        ///
        /// - if any of the `market_authorities` addresses are invalid
        /// - if the `admin` address is invalid
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
        #[must_use]
        pub fn market_authority_parse_error(err: AddressError) -> Self {
            Self(ParamsErrorKind::MarketAuthorityParseError(err))
        }

        #[must_use]
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
        pub ticker: Ticker,
        pub provider_configs: Vec<ProviderConfig>,
    }

    impl Market {
        /// Converts from a raw protobuf `Market` to a native `Market`.
        ///
        /// # Errors
        ///
        /// - if the `ticker` field is missing
        /// - if the `ticker` field is invalid
        /// - if any of the `provider_configs` are invalid
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
        #[must_use]
        pub fn missing_ticker() -> Self {
            Self(MarketErrorKind::MissingTicker)
        }

        #[must_use]
        pub fn invalid_ticker(err: TickerError) -> Self {
            Self(MarketErrorKind::TickerParseError(err))
        }

        #[must_use]
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
        pub currency_pair: CurrencyPair,
        pub decimals: u64,
        pub min_provider_count: u64,
        pub enabled: bool,
        pub metadata_json: String,
    }

    impl Ticker {
        /// Converts from a raw protobuf `Ticker` to a native `Ticker`.
        ///
        /// # Errors
        ///
        /// - if the `currency_pair` field is missing
        /// - if the `currency_pair` field is invalid
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
        #[must_use]
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
        pub name: String,
        pub off_chain_ticker: String,
        pub normalize_by_pair: CurrencyPair,
        pub invert: bool,
        pub metadata_json: String,
    }

    impl ProviderConfig {
        /// Converts from a raw protobuf `ProviderConfig` to a native `ProviderConfig`.
        ///
        /// # Errors
        ///
        /// - if the `normalize_by_pair` field is missing
        pub fn try_from_raw(raw: raw::ProviderConfig) -> Result<Self, ProviderConfigError> {
            let Some(normalize_by_pair) = raw.normalize_by_pair.map(CurrencyPair::from_raw) else {
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
        #[must_use]
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
        /// Converts from a raw protobuf `MarketMap` to a native `MarketMap`.
        ///
        /// # Errors
        ///
        /// - if any of the markets are invalid
        /// - if any of the market names are invalid
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
        #[must_use]
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
