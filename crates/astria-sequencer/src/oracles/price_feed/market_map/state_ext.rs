use astria_core::oracles::price_feed::market_map::v2::{
    MarketMap,
    Params,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_market_map(&self) -> Result<Option<MarketMap>> {
        let Some(bytes) = self
            .get_raw(keys::MARKET_MAP)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get market map from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::MarketMap::try_from(value)
                    .map(|market_map| Some(MarketMap::from(market_map)))
            })
            .wrap_err("invalid market map bytes")
    }

    #[instrument(skip_all)]
    async fn get_market_map_last_updated_height(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::LAST_UPDATED)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading market map last updated height from state")?
        else {
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::BlockHeight::try_from(value).map(u64::from))
            .wrap_err("invalid updated height bytes")
    }

    #[instrument(skip_all)]
    async fn get_params(&self) -> Result<Option<Params>> {
        let Some(bytes) = self
            .get_raw(keys::PARAMS)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get params from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::Params::try_from(value).map(|params| Some(Params::from(params)))
            })
            .wrap_err("invalid params bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_market_map(&mut self, market_map: MarketMap) -> Result<()> {
        let bytes = StoredValue::from(storage::MarketMap::from(&market_map))
            .serialize()
            .wrap_err("failed to serialize market map")?;
        self.put_raw(keys::MARKET_MAP.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_market_map_last_updated_height(&mut self, height: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::BlockHeight::from(height))
            .serialize()
            .wrap_err("failed to serialize last updated height")?;
        self.put_raw(keys::LAST_UPDATED.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_params(&mut self, params: Params) -> Result<()> {
        let bytes = StoredValue::from(storage::Params::from(&params))
            .serialize()
            .wrap_err("failed to serialize params")?;
        self.put_raw(keys::PARAMS.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::generated::price_feed::{
        marketmap::v2 as raw,
        types::v2::CurrencyPair,
    };
    use cnidarium::StateDelta;
    use serde_json::{
        json,
        Value,
    };

    use super::*;
    use crate::{
        app::benchmark_and_test_utils::{
            ALICE_ADDRESS,
            BOB_ADDRESS,
            CAROL_ADDRESS,
            JUDY_ADDRESS,
        },
        benchmark_and_test_utils::astria_address_from_hex_string,
    };

    /// Returns a `MarketMap` with the provided metadata encoded into the first market's ticker to
    /// support creating non-identical maps.
    fn market_map(metadata: Option<Value>) -> MarketMap {
        let raw_market_map = raw::MarketMap {
            markets: [
                (
                    "BTC/USD".to_string(),
                    raw::Market {
                        ticker: Some(raw::Ticker {
                            currency_pair: Some(CurrencyPair {
                                base: "BTC".to_string(),
                                quote: "USD".to_string(),
                            }),
                            decimals: 8,
                            min_provider_count: 3,
                            enabled: true,
                            metadata_json: metadata
                                .map(|value| value.to_string())
                                .unwrap_or_default(),
                        }),
                        provider_configs: vec![raw::ProviderConfig {
                            name: "coingecko_api".to_string(),
                            off_chain_ticker: "bitcoin/usd".to_string(),
                            normalize_by_pair: Some(CurrencyPair {
                                base: "USDT".to_string(),
                                quote: "USD".to_string(),
                            }),
                            invert: false,
                            metadata_json: json!({ "field": true }).to_string(),
                        }],
                    },
                ),
                (
                    "ETH/USD".to_string(),
                    raw::Market {
                        ticker: Some(raw::Ticker {
                            currency_pair: Some(CurrencyPair {
                                base: "ETH".to_string(),
                                quote: "USD".to_string(),
                            }),
                            decimals: 8,
                            min_provider_count: 3,
                            enabled: true,
                            metadata_json: String::new(),
                        }),
                        provider_configs: vec![raw::ProviderConfig {
                            name: "coingecko_api".to_string(),
                            off_chain_ticker: "ethereum/usd".to_string(),
                            normalize_by_pair: Some(CurrencyPair {
                                base: "USDT".to_string(),
                                quote: "USD".to_string(),
                            }),
                            invert: false,
                            metadata_json: String::new(),
                        }],
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };
        MarketMap::try_from(raw_market_map).unwrap()
    }

    /// Returns a `Params` with the provided addresses as the authorities, and the first one used as
    /// the admin.
    fn params(addresses: impl IntoIterator<Item = &'static str>) -> Params {
        let market_authorities: Vec<_> = addresses
            .into_iter()
            .map(astria_address_from_hex_string)
            .collect();
        let admin = *market_authorities.first().unwrap();
        Params {
            market_authorities,
            admin,
        }
    }

    #[tokio::test]
    async fn should_put_and_get_market_map() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Getting should return `None` when no market map is stored.
        assert!(state.get_market_map().await.unwrap().is_none());

        // Putting a market map should succeed.
        let market_map_1 = market_map(Some(json!({ "field": 1 })));
        state.put_market_map(market_map_1.clone()).unwrap();

        // Getting the stored market map should succeed.
        let retrieved_market_map = state
            .get_market_map()
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(market_map_1, retrieved_market_map);

        // Putting a new market map should overwrite the first.
        let market_map_2 = market_map(None);
        assert_ne!(market_map_1, market_map_2);
        state.put_market_map(market_map_2.clone()).unwrap();

        let retrieved_market_map = state
            .get_market_map()
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(market_map_2, retrieved_market_map);
    }

    #[tokio::test]
    async fn should_put_and_get_market_map_last_updated_height() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Getting should return `0` when no height is stored.
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 0);

        // Putting a height should succeed.
        state.put_market_map_last_updated_height(1).unwrap();

        // Getting the stored height should succeed.
        let retrieved_height = state
            .get_market_map_last_updated_height()
            .await
            .expect("should not error");
        assert_eq!(1, retrieved_height);

        // Putting a new height should overwrite the first.
        state.put_market_map_last_updated_height(2).unwrap();

        let retrieved_height = state
            .get_market_map_last_updated_height()
            .await
            .expect("should not error");
        assert_eq!(2, retrieved_height);
    }

    #[tokio::test]
    async fn should_put_and_get_params() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Getting should return `None` when no params are stored.
        assert!(state.get_params().await.unwrap().is_none());

        // Putting params should succeed.
        let params_1 = params([ALICE_ADDRESS, BOB_ADDRESS, CAROL_ADDRESS]);
        state.put_params(params_1.clone()).unwrap();

        // Getting the stored params should succeed.
        let retrieved_params = state
            .get_params()
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(params_1, retrieved_params);

        // Putting new params should overwrite the first.
        let params_2 = params([BOB_ADDRESS, JUDY_ADDRESS]);
        assert_ne!(params_1, params_2);
        state.put_params(params_2.clone()).unwrap();

        let retrieved_params = state
            .get_params()
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(params_2, retrieved_params);
    }
}
