use std::collections::BTreeMap;

use astria_core_address::Address;

use super::v1::Upgrades;
use crate::generated::{
    price_feed::{
        marketmap::v2::{
            GenesisState as RawMarketMapGenesisState,
            Market as RawMarket,
            MarketMap as RawMarketMap,
            Params as RawMarketMapParams,
            ProviderConfig as RawProviderConfig,
            Ticker as RawTicker,
        },
        oracle::v2::{
            CurrencyPairGenesis as RawCurrencyPairGenesis,
            GenesisState as RawOracleGenesisState,
            QuotePrice as RawQuotePrice,
        },
        types::v2::CurrencyPair as RawCurrencyPair,
    },
    upgrades::v1 as raw,
};

pub struct UpgradesBuilder {
    aspen_activation_height: Option<u64>,
}

impl UpgradesBuilder {
    /// Returns a new `UpgradesBuilder`.
    ///
    /// By default, Aspen is included with an activation height of 100.
    #[must_use]
    pub fn new() -> Self {
        Self {
            aspen_activation_height: Some(100),
        }
    }

    /// To exclude Aspen, provide `activation_height` as `None`.
    #[must_use]
    pub fn set_aspen(mut self, activation_height: Option<u64>) -> Self {
        self.aspen_activation_height = activation_height;
        self
    }

    #[must_use]
    pub fn build(self) -> Upgrades {
        let aspen = self
            .aspen_activation_height
            .map(|activation_height| raw::Aspen {
                base_info: Some(raw::BaseUpgradeInfo {
                    activation_height,
                    app_version: 2,
                }),
                price_feed_change: Some(raw::aspen::PriceFeedChange {
                    market_map_genesis: Some(market_map_genesis()),
                    oracle_genesis: Some(oracle_genesis()),
                }),
                validator_update_action_change: Some(raw::aspen::ValidatorUpdateActionChange {}),
                ibc_acknowledgement_failure_change: Some(
                    raw::aspen::IbcAcknowledgementFailureChange {},
                ),
            });
        let raw_upgrades = raw::Upgrades {
            aspen,
        };
        Upgrades::from_raw(raw_upgrades)
    }
}

impl Default for UpgradesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn market_map_genesis() -> RawMarketMapGenesisState {
    let mut markets = BTreeMap::new();
    markets.insert(
        "BTC/USD".to_string(),
        RawMarket {
            ticker: Some(RawTicker {
                currency_pair: Some(RawCurrencyPair {
                    base: "BTC".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![RawProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "bitcoin/usd".to_string(),
                normalize_by_pair: None,
                invert: false,
                metadata_json: String::new(),
            }],
        },
    );
    markets.insert(
        "ETH/USD".to_string(),
        RawMarket {
            ticker: Some(RawTicker {
                currency_pair: Some(RawCurrencyPair {
                    base: "ETH".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![RawProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "ethereum/usd".to_string(),
                normalize_by_pair: None,
                invert: false,
                metadata_json: String::new(),
            }],
        },
    );

    RawMarketMapGenesisState {
        market_map: Some(RawMarketMap {
            markets,
        }),
        last_updated: 0,
        params: Some(RawMarketMapParams {
            market_authorities: vec![alice().to_string(), bob().to_string()],
            admin: alice().to_string(),
        }),
    }
}

fn oracle_genesis() -> RawOracleGenesisState {
    RawOracleGenesisState {
        currency_pair_genesis: vec![
            RawCurrencyPairGenesis {
                id: 0,
                nonce: 0,
                currency_pair_price: Some(RawQuotePrice {
                    price: 5_834_065_777_u128.to_string(),
                    block_height: 0,
                    block_timestamp: Some(pbjson_types::Timestamp {
                        seconds: 1_720_122_395,
                        nanos: 0,
                    }),
                }),
                currency_pair: Some(RawCurrencyPair {
                    base: "BTC".to_string(),
                    quote: "USD".to_string(),
                }),
            },
            RawCurrencyPairGenesis {
                id: 1,
                nonce: 0,
                currency_pair_price: Some(RawQuotePrice {
                    price: 3_138_872_234_u128.to_string(),
                    block_height: 0,
                    block_timestamp: Some(pbjson_types::Timestamp {
                        seconds: 1_720_122_395,
                        nanos: 0,
                    }),
                }),
                currency_pair: Some(RawCurrencyPair {
                    base: "ETH".to_string(),
                    quote: "USD".to_string(),
                }),
            },
        ],
        next_id: 2,
    }
}

fn alice() -> Address {
    Address::builder()
        .prefix("astria")
        .slice(hex::decode("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap())
        .try_build()
        .unwrap()
}

fn bob() -> Address {
    Address::builder()
        .prefix("astria")
        .slice(hex::decode("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a").unwrap())
        .try_build()
        .unwrap()
}
