use astria_core::{
    connect::market_map::v2::MarketMap,
    protocol::transaction::v1::action::ChangeMarkets::{
        self,
        Create,
        Remove,
        Update,
    },
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    Context as _,
    OptionExt as _,
};
use cnidarium::StateWrite;
use indexmap::IndexMap;

use crate::{
    action_handler::ActionHandler,
    address::StateReadExt as _,
    app::StateReadExt as _,
    connect::market_map::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for ChangeMarkets {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        // check that the signer of the transaction is a market authority
        let from = state
            .try_base_prefixed(
                &state
                    .get_transaction_context()
                    .expect("transaction source must be present in state when executing an action")
                    .address_bytes(),
            )
            .await
            .wrap_err("failed to convert signer address to base prefixed address")?;
        let market_authorities = state
            .get_params()
            .await?
            .ok_or_eyre("market map params not found in state")?
            .market_authorities;
        ensure!(
            market_authorities.contains(&from),
            "address {from} is not a market authority"
        );

        // create a new market map if one does not already exist
        let mut market_map = state
            .get_market_map()
            .await
            .wrap_err("failed to get market map")?
            .unwrap_or(MarketMap {
                markets: IndexMap::new(),
            });
        match self {
            Create(create_markets) => {
                for market in create_markets {
                    let ticker_key = market.ticker.currency_pair.to_string();
                    if market_map.markets.contains_key(&ticker_key) {
                        bail!("market for ticker {ticker_key} already exists");
                    }
                    market_map.markets.insert(ticker_key, market.clone());
                }
            }
            Update(update_markets) => {
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
            Remove(remove_markets) => {
                for key in remove_markets {
                    market_map
                        .markets
                        .shift_remove(&key.ticker.currency_pair.to_string());
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
mod tests {

    use astria_core::{
        connect::market_map::v2::{
            Market,
            Params,
        },
        primitive::v1::TransactionId,
    };

    use super::*;
    use crate::{
        accounts::AddressBytes,
        address::StateWriteExt as _,
        app::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        test_utils::{
            example_ticker_from_currency_pair,
            example_ticker_with_metadata,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn create_markets_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

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

        let action = ChangeMarkets::Create(vec![market.clone()]);
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

        let params = Params {
            market_authorities: vec![], // should fail even though the authority address is admin
            admin: authority_address,
        };
        state.put_params(params).unwrap();

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = ChangeMarkets::Create(vec![]);
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains(&format!(
            "address {authority_address} is not a market authority"
        )));
        assert!(state.get_market_map().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn change_markets_fails_if_params_not_found() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = ChangeMarkets::Create(vec![]);
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(
            res.to_string()
                .contains("market map params not found in state")
        );
        assert!(state.get_market_map().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn remove_markets_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);

        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

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

        let action = ChangeMarkets::Remove(vec![Market {
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

        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

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

        let action = ChangeMarkets::Remove(vec![Market {
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

        state.put_block_height(1).unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

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

        let action = ChangeMarkets::Update(vec![market.clone()]);

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

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

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

        let action = ChangeMarkets::Update(vec![market.clone()]);

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains(&format!(
            "market for ticker {} not found in market map",
            ticker.currency_pair
        )));
    }

    #[tokio::test]
    async fn update_markets_fails_if_market_map_is_not_in_state() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

        let action = ChangeMarkets::Update(vec![]);

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map is empty"));
    }
}
