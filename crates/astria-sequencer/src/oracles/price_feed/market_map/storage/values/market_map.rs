use std::borrow::Cow;

use astria_core::oracles::price_feed::{
    market_map::v2::{
        Market as DomainMarket,
        MarketMap as DomainMarketMap,
        ProviderConfig as DomainProviderConfig,
        Ticker as DomainTicker,
    },
    types::v2::{
        Base,
        CurrencyPair as DomainCurrencyPair,
        Quote,
    },
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct CurrencyPair<'a> {
    base: Cow<'a, str>,
    quote: Cow<'a, str>,
}

impl<'a> From<&'a DomainCurrencyPair> for CurrencyPair<'a> {
    fn from(currency_pair: &'a DomainCurrencyPair) -> Self {
        CurrencyPair {
            base: Cow::Borrowed(currency_pair.base()),
            quote: Cow::Borrowed(currency_pair.quote()),
        }
    }
}

impl<'a> From<CurrencyPair<'a>> for DomainCurrencyPair {
    fn from(currency_pair: CurrencyPair<'a>) -> Self {
        DomainCurrencyPair::from_parts(
            Base::unchecked_from_parts(currency_pair.base.into_owned()),
            Quote::unchecked_from_parts(currency_pair.quote.into_owned()),
        )
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Ticker<'a> {
    currency_pair: CurrencyPair<'a>,
    decimals: u8,
    min_provider_count: u64,
    enabled: bool,
    metadata_json: Cow<'a, str>,
}

impl<'a> From<&'a DomainTicker> for Ticker<'a> {
    fn from(ticker: &'a DomainTicker) -> Self {
        Ticker {
            currency_pair: CurrencyPair::from(&ticker.currency_pair),
            decimals: ticker.decimals,
            min_provider_count: ticker.min_provider_count,
            enabled: ticker.enabled,
            metadata_json: Cow::Borrowed(&ticker.metadata_json),
        }
    }
}

impl<'a> From<Ticker<'a>> for DomainTicker {
    fn from(ticker: Ticker<'a>) -> Self {
        DomainTicker::unchecked_from_parts(
            DomainCurrencyPair::from(ticker.currency_pair),
            ticker.decimals,
            ticker.min_provider_count,
            ticker.enabled,
            ticker.metadata_json.into_owned(),
        )
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct ProviderConfig<'a> {
    name: Cow<'a, str>,
    off_chain_ticker: Cow<'a, str>,
    normalize_by_pair: Option<CurrencyPair<'a>>,
    invert: bool,
    metadata_json: Cow<'a, str>,
}

impl<'a> From<&'a DomainProviderConfig> for ProviderConfig<'a> {
    fn from(provider_config: &'a DomainProviderConfig) -> Self {
        ProviderConfig {
            name: Cow::Borrowed(&provider_config.name),
            off_chain_ticker: Cow::Borrowed(&provider_config.off_chain_ticker),
            normalize_by_pair: provider_config
                .normalize_by_pair
                .as_ref()
                .map(CurrencyPair::from),
            invert: provider_config.invert,
            metadata_json: Cow::Borrowed(&provider_config.metadata_json),
        }
    }
}

impl<'a> From<ProviderConfig<'a>> for DomainProviderConfig {
    fn from(provider_config: ProviderConfig<'a>) -> Self {
        DomainProviderConfig::unchecked_from_parts(
            provider_config.name.into_owned(),
            provider_config.off_chain_ticker.into_owned(),
            provider_config
                .normalize_by_pair
                .map(DomainCurrencyPair::from),
            provider_config.invert,
            provider_config.metadata_json.into_owned(),
        )
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Market<'a> {
    name: Cow<'a, str>,
    ticker: Ticker<'a>,
    provider_configs: Vec<ProviderConfig<'a>>,
}

impl<'a> From<(&'a String, &'a DomainMarket)> for Market<'a> {
    fn from((name, market): (&'a String, &'a DomainMarket)) -> Self {
        Market {
            name: Cow::Borrowed(name.as_str()),
            ticker: Ticker::from(&market.ticker),
            provider_configs: market
                .provider_configs
                .iter()
                .map(ProviderConfig::from)
                .collect(),
        }
    }
}

impl<'a> From<Market<'a>> for (String, DomainMarket) {
    fn from(market: Market<'a>) -> Self {
        let domain_market = DomainMarket::unchecked_from_parts(
            DomainTicker::from(market.ticker),
            market
                .provider_configs
                .into_iter()
                .map(DomainProviderConfig::from)
                .collect(),
        );
        (market.name.into_owned(), domain_market)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::oracles::price_feed::market_map) struct MarketMap<'a>(Vec<Market<'a>>);

impl<'a> From<&'a DomainMarketMap> for MarketMap<'a> {
    fn from(market_map: &'a DomainMarketMap) -> Self {
        MarketMap(market_map.markets.iter().map(Market::from).collect())
    }
}

impl<'a> From<MarketMap<'a>> for DomainMarketMap {
    fn from(market_map: MarketMap<'a>) -> Self {
        DomainMarketMap::unchecked_from_parts(
            market_map.0.into_iter().map(<(String, DomainMarket)>::from),
        )
    }
}

impl<'a> From<MarketMap<'a>> for crate::storage::StoredValue<'a> {
    fn from(market_map: MarketMap<'a>) -> Self {
        crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::MarketMap(market_map)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for MarketMap<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::MarketMap(
            market_map,
        ))) = value
        else {
            bail!(
                "price feed market map stored value type mismatch: expected market map, found \
                 {value:?}"
            );
        };
        Ok(market_map)
    }
}
