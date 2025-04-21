use astria_core::protocol::transaction::v1::action::MarketsChange;
use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    action_handler::ActionHandler,
    app::StateReadExt as _,
    authority::StateReadExt as _,
    oracles::price_feed::market_map::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for MarketsChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        let mut market_map = state
            .get_market_map()
            .await
            .wrap_err("failed to get market map")?
            .ok_or_eyre("market map not found in state")?;
        match self {
            MarketsChange::Creation(create_markets) => {
                for market in create_markets {
                    let ticker_key = market.ticker.currency_pair.to_string();
                    if market_map.markets.contains_key(&ticker_key) {
                        bail!("market for ticker {ticker_key} already exists");
                    }
                    market_map.markets.insert(ticker_key, market.clone());
                }
            }
            MarketsChange::Removal(remove_markets) => {
                for key in remove_markets {
                    market_map
                        .markets
                        .shift_remove(&key.ticker.currency_pair.to_string());
                }
            }
            MarketsChange::Update(update_markets) => {
                if market_map.markets.is_empty() {
                    bail!("market map is empty");
                }
                for market in update_markets {
                    let ticker_key = market.ticker.currency_pair.to_string();
                    *market_map.markets.get_mut(&ticker_key).ok_or_else(|| {
                        eyre!("market for ticker {ticker_key} not found in market map")
                    })? = market.clone();
                }
            }
        };

        state
            .put_market_map(market_map)
            .wrap_err("failed to put market map into state")?;

        // update the last updated height for the market map
        state
            .put_market_map_last_updated_height(
                state
                    .get_block_height()
                    .await
                    .wrap_err("failed to get block height")?,
            )
            .wrap_err("failed to update latest market map height")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        oracles::price_feed::market_map::v2::{
            Market,
            MarketMap,
        },
        primitive::v1::TransactionId,
    };
    use indexmap::IndexMap;

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        app::StateWriteExt as _,
        authority::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        test_utils::{
            example_ticker_from_currency_pair,
            example_ticker_with_metadata,
        },
        transaction::{
            StateWriteExt,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn create_markets_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state
            .put_market_map(MarketMap {
                markets: IndexMap::new(),
            })
            .unwrap();

        let ticker = example_ticker_with_metadata(String::new());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let action = MarketsChange::Creation(vec![market.clone()]);
        action.check_and_execute(&mut state).await.unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 1);
        assert_eq!(
            *market_map
                .markets
                .get(&ticker.currency_pair.to_string())
                .unwrap(),
            market,
        );
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn change_markets_fails_if_authority_is_invalid() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = MarketsChange::Creation(vec![]);
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("signer is not the sudo key"));
        assert!(state.get_market_map().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn remove_markets_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let ticker = example_ticker_with_metadata(String::new());

        let mut markets = IndexMap::new();
        markets.insert(
            ticker.currency_pair.to_string(),
            Market {
                ticker: ticker.clone(),
                provider_configs: vec![],
            },
        );

        state
            .put_market_map(MarketMap {
                markets,
            })
            .unwrap();

        let action = MarketsChange::Removal(vec![Market {
            ticker,
            provider_configs: vec![],
        }]);
        action.check_and_execute(&mut state).await.unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 0);
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn remove_markets_skips_missing_markets() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let ticker = example_ticker_with_metadata(String::new());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let mut markets = IndexMap::new();
        markets.insert(ticker.currency_pair.to_string(), market.clone());

        state
            .put_market_map(MarketMap {
                markets,
            })
            .unwrap();

        let action = MarketsChange::Removal(vec![Market {
            ticker: example_ticker_from_currency_pair("DIFBASE", "DIFQUOTE", String::new()),
            provider_configs: vec![],
        }]);
        action.check_and_execute(&mut state).await.unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 1);
        assert_eq!(
            market_map.markets.get(&ticker.currency_pair.to_string()),
            Some(&market)
        );
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn update_markets_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let ticker_1 = example_ticker_with_metadata("ticker_1".to_string());
        let market_1 = Market {
            ticker: ticker_1.clone(),
            provider_configs: vec![],
        };

        let mut markets = IndexMap::new();
        markets.insert(ticker_1.currency_pair.to_string(), market_1.clone());
        let initial_market_map = MarketMap {
            markets,
        };
        state.put_market_map(initial_market_map).unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 1);
        assert_eq!(
            *market_map
                .markets
                .get(&ticker_1.currency_pair.to_string())
                .unwrap(),
            market_1,
        );

        let ticker = example_ticker_with_metadata("ticker".to_string());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let action = MarketsChange::Update(vec![market.clone()]);

        action.check_and_execute(&mut state).await.unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 1);
        assert_eq!(
            *market_map
                .markets
                .get(&ticker.currency_pair.to_string())
                .unwrap(),
            market,
        );
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn update_markets_fails_if_market_is_not_in_market_map() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let ticker = example_ticker_with_metadata("ticker".to_string());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let different_ticker = example_ticker_from_currency_pair(
            "difbase",
            "difquote",
            "different ticker".to_string(),
        );
        let different_market = Market {
            ticker: different_ticker.clone(),
            provider_configs: vec![],
        };
        let mut market_map = MarketMap {
            markets: IndexMap::new(),
        };
        market_map.markets.insert(
            different_ticker.currency_pair.to_string(),
            different_market.clone(),
        );
        state.put_market_map(market_map).unwrap();

        let action = MarketsChange::Update(vec![market.clone()]);

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains(&format!(
            "market for ticker {} not found in market map",
            ticker.currency_pair
        )));
    }

    #[tokio::test]
    async fn market_map_action_fails_if_market_map_is_not_in_state() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = MarketsChange::Update(vec![]);

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map not found in state"));
    }

    #[tokio::test]
    async fn update_markets_fails_if_market_map_is_empty() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        state.put_sudo_address(authority_address).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state
            .put_market_map(MarketMap {
                markets: IndexMap::new(),
            })
            .unwrap();

        let action = MarketsChange::Update(vec![]);

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map is empty"));
    }
}
