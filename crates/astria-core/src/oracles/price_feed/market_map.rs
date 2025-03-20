pub mod v2 {
    use std::{
        io::{
            self,
            Write,
        },
        str::FromStr,
    };

    use borsh::BorshSerialize;
    use indexmap::IndexMap;

    use crate::{
        generated::price_feed::marketmap::v2 as raw,
        oracles::price_feed::types::v2::{
            CurrencyPair,
            CurrencyPairError,
        },
        primitive::v1::{
            Address,
            AddressError,
        },
        Protobuf,
    };

    #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize)]
    pub struct GenesisState {
        pub market_map: MarketMap,
        pub last_updated: u64,
        pub params: Params,
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

        fn try_from_raw_ref(raw: &raw::GenesisState) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw.clone())
        }

        /// Converts from a raw protobuf `GenesisState` to a native `GenesisState`.
        ///
        /// # Errors
        ///
        /// - if the `market_map` field is missing
        /// - if the `market_map` field is invalid
        /// - if the `params` field is missing
        /// - if the `params` field is invalid
        fn try_from_raw(raw: raw::GenesisState) -> Result<Self, GenesisStateError> {
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

        fn to_raw(&self) -> raw::GenesisState {
            self.clone().into_raw()
        }

        #[must_use]
        fn into_raw(self) -> raw::GenesisState {
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
        #[error("failed to parse market map")]
        MarketMapParseError(#[from] MarketMapError),
        #[error("missing params")]
        MissingParams,
        #[error("failed to parse params")]
        ParamsParseError(#[from] ParamsError),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Params {
        pub market_authorities: Vec<Address>,
        pub admin: Address,
    }

    impl TryFrom<raw::Params> for Params {
        type Error = ParamsError;

        fn try_from(raw: raw::Params) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<Params> for raw::Params {
        fn from(params: Params) -> Self {
            params.into_raw()
        }
    }

    impl BorshSerialize for Params {
        fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            let market_authorities: Vec<_> = self
                .market_authorities
                .iter()
                .map(Address::to_string)
                .collect();
            (market_authorities, self.admin.to_string()).serialize(writer)
        }
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

        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(market_authorities: Vec<Address>, admin: Address) -> Self {
            Self {
                market_authorities,
                admin,
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

    #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize)]
    pub struct Market {
        pub ticker: Ticker,
        pub provider_configs: Vec<ProviderConfig>,
    }

    impl TryFrom<raw::Market> for Market {
        type Error = MarketError;

        fn try_from(raw: raw::Market) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<Market> for raw::Market {
        fn from(market: Market) -> Self {
            market.into_raw()
        }
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

        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(ticker: Ticker, provider_configs: Vec<ProviderConfig>) -> Self {
            Self {
                ticker,
                provider_configs,
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
        #[error("failed to parse ticker")]
        TickerParseError(#[from] TickerError),
        #[error("failed to parse provider config")]
        ProviderConfigParseError(#[from] ProviderConfigError),
    }

    #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize)]
    pub struct Ticker {
        pub currency_pair: CurrencyPair,
        pub decimals: u8,
        pub min_provider_count: u64,
        pub enabled: bool,
        pub metadata_json: String,
    }

    impl TryFrom<raw::Ticker> for Ticker {
        type Error = TickerError;

        fn try_from(raw: raw::Ticker) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<Ticker> for raw::Ticker {
        fn from(ticker: Ticker) -> Self {
            ticker.into_raw()
        }
    }

    impl Ticker {
        /// Converts from a raw protobuf `Ticker` to a native `Ticker`.
        ///
        /// # Errors
        ///
        /// - if the `currency_pair` field is missing
        /// - if the `currency_pair` field is invalid
        pub fn try_from_raw(raw: raw::Ticker) -> Result<Self, TickerError> {
            let currency_pair = raw
                .currency_pair
                .ok_or_else(|| TickerError::field_not_set("currency_pair"))?
                .try_into()
                .map_err(TickerError::invalid_currency_pair)?;
            let decimals = raw
                .decimals
                .try_into()
                .map_err(|_| TickerError::decimals_too_large())?;

            Ok(Self {
                currency_pair,
                decimals,
                min_provider_count: raw.min_provider_count,
                enabled: raw.enabled,
                metadata_json: raw.metadata_json,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::Ticker {
            raw::Ticker {
                currency_pair: Some(self.currency_pair.into_raw()),
                decimals: self.decimals.into(),
                min_provider_count: self.min_provider_count,
                enabled: self.enabled,
                metadata_json: self.metadata_json,
            }
        }

        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(
            currency_pair: CurrencyPair,
            decimals: u8,
            min_provider_count: u64,
            enabled: bool,
            metadata_json: String,
        ) -> Self {
            Self {
                currency_pair,
                decimals,
                min_provider_count,
                enabled,
                metadata_json,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct TickerError(#[from] TickerErrorKind);

    impl TickerError {
        #[must_use]
        fn field_not_set(name: &'static str) -> Self {
            TickerErrorKind::FieldNotSet {
                name,
            }
            .into()
        }

        #[must_use]
        fn invalid_currency_pair(source: CurrencyPairError) -> Self {
            TickerErrorKind::InvalidCurrencyPair {
                source,
            }
            .into()
        }

        #[must_use]
        fn decimals_too_large() -> Self {
            TickerErrorKind::DecimalsTooLarge.into()
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("failed validating wire type `{}`", raw::Ticker::full_name())]
    enum TickerErrorKind {
        #[error("required field not set: .{name}")]
        FieldNotSet { name: &'static str },
        #[error("field `.currency_pair` was invalid")]
        InvalidCurrencyPair { source: CurrencyPairError },
        #[error("field `.decimals` was too large; must fit in a u8")]
        DecimalsTooLarge,
    }

    #[derive(Debug, Clone, PartialEq, Eq, BorshSerialize)]
    pub struct ProviderConfig {
        pub name: String,
        pub off_chain_ticker: String,
        pub normalize_by_pair: Option<CurrencyPair>,
        pub invert: bool,
        pub metadata_json: String,
    }

    impl TryFrom<raw::ProviderConfig> for ProviderConfig {
        type Error = ProviderConfigError;

        fn try_from(raw: raw::ProviderConfig) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<ProviderConfig> for raw::ProviderConfig {
        fn from(provider_config: ProviderConfig) -> Self {
            provider_config.into_raw()
        }
    }

    impl ProviderConfig {
        /// Converts from a raw protobuf `ProviderConfig` to a native `ProviderConfig`.
        ///
        /// # Errors
        ///
        /// - if the `normalize_by_pair` field is missing
        pub fn try_from_raw(raw: raw::ProviderConfig) -> Result<Self, ProviderConfigError> {
            let normalize_by_pair = raw
                .normalize_by_pair
                .map(CurrencyPair::try_from_raw)
                .transpose()
                .map_err(ProviderConfigError::invalid_normalize_by_pair)?;
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
                normalize_by_pair: self.normalize_by_pair.map(CurrencyPair::into_raw),
                invert: self.invert,
                metadata_json: self.metadata_json,
            }
        }

        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts(
            name: String,
            off_chain_ticker: String,
            normalize_by_pair: Option<CurrencyPair>,
            invert: bool,
            metadata_json: String,
        ) -> Self {
            Self {
                name,
                off_chain_ticker,
                normalize_by_pair,
                invert,
                metadata_json,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct ProviderConfigError(#[from] ProviderConfigErrorKind);

    impl ProviderConfigError {
        fn invalid_normalize_by_pair(source: CurrencyPairError) -> Self {
            ProviderConfigErrorKind::InvalidNormalizeByPair {
                source,
            }
            .into()
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("failed validating wire type `{}`", raw::ProviderConfig::full_name())]
    enum ProviderConfigErrorKind {
        #[error("field `.normalize_by_pair` was invalid")]
        InvalidNormalizeByPair { source: CurrencyPairError },
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MarketMap {
        pub markets: IndexMap<String, Market>,
    }

    impl TryFrom<raw::MarketMap> for MarketMap {
        type Error = MarketMapError;

        fn try_from(raw: raw::MarketMap) -> Result<Self, Self::Error> {
            Self::try_from_raw(raw)
        }
    }

    impl From<MarketMap> for raw::MarketMap {
        fn from(market_map: MarketMap) -> Self {
            market_map.into_raw()
        }
    }

    impl BorshSerialize for MarketMap {
        fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            u32::try_from(self.markets.len())
                .map_err(|_| io::ErrorKind::InvalidInput)?
                .serialize(writer)?;
            for (key, value) in &self.markets {
                key.serialize(writer)?;
                value.serialize(writer)?;
            }
            Ok(())
        }
    }

    impl MarketMap {
        /// Converts from a raw protobuf `MarketMap` to a native `MarketMap`.
        ///
        /// # Errors
        ///
        /// - if any of the markets are invalid
        /// - if any of the market names are invalid
        pub fn try_from_raw(raw: raw::MarketMap) -> Result<Self, MarketMapError> {
            let mut markets = IndexMap::new();
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

        /// This should only be used where the inputs have been provided by a trusted entity, e.g.
        /// read from our own state store.
        ///
        /// Note that this function is not considered part of the public API and is subject to
        /// breaking change at any time.
        #[cfg(feature = "unchecked-constructors")]
        #[doc(hidden)]
        #[must_use]
        pub fn unchecked_from_parts<I: IntoIterator<Item = (String, Market)>>(
            name_and_market_iter: I,
        ) -> Self {
            Self {
                markets: IndexMap::from_iter(name_and_market_iter),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct MarketMapError(MarketMapErrorKind);

    impl MarketMapError {
        #[must_use]
        pub fn invalid_market(name: String, err: MarketError) -> Self {
            Self(MarketMapErrorKind::InvalidMarket {
                name,
                source: err,
            })
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum MarketMapErrorKind {
        #[error("invalid market `{name}`")]
        InvalidMarket { name: String, source: MarketError },
    }
}
