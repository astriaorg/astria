use astria_core::{
    connect::market_map::v2::MarketMap,
    protocol::transaction::v1::action::UpsertMarkets,
};
use astria_eyre::eyre::{
    self,
    ensure,
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
impl ActionHandler for UpsertMarkets {
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

        // upsert markets, updating any that already exist and creating any that do not
        let mut market_map = state
            .get_market_map()
            .await
            .wrap_err("failed to get market map")?
            .unwrap_or(MarketMap {
                markets: IndexMap::new(),
            });
        for market in &self.markets {
            let ticker_key = market.ticker.currency_pair.to_string();
            market_map
                .markets
                .entry(ticker_key)
                .and_modify(|existing_market| {
                    *existing_market = market.clone();
                })
                .or_insert(market.clone());
        }
        state
            .put_market_map(market_map)
            .wrap_err("failed to put market map into state")?;

        // update the last updated height of the market map
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
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        app::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        test_utils::example_ticker_with_metadata,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn upsert_markets_creates_market_as_expected() {
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

        let action = UpsertMarkets {
            markets: vec![market.clone()],
        };
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
    async fn upsert_markets_updates_market_if_it_already_exists() {
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

        let ticker_2 = example_ticker_with_metadata("ticker_2".to_string());
        let market_2 = Market {
            ticker: ticker_2.clone(),
            provider_configs: vec![],
        };

        let action = UpsertMarkets {
            markets: vec![market_2.clone()],
        };

        action.check_and_execute(&mut state).await.unwrap();
        let market_map = state.get_market_map().await.unwrap().unwrap();
        assert_eq!(market_map.markets.len(), 1);
        assert_eq!(
            *market_map
                .markets
                .get(&ticker_2.currency_pair.to_string())
                .unwrap(),
            market_2,
        );
        assert_eq!(state.get_market_map_last_updated_height().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn upsert_markets_fails_if_authority_is_invalid() {
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
            market_authorities: vec![], // should fail even though the authority address is admin
            admin: authority_address,
        };
        state.put_params(params).unwrap();

        let action = UpsertMarkets {
            markets: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains(&format!(
            "address {authority_address} is not a market authority"
        )));
        assert!(state.get_market_map().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn upsert_markets_fails_if_params_not_found() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = UpsertMarkets {
            markets: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(
            res.to_string()
                .contains("market map params not found in state")
        );
        assert!(state.get_market_map().await.unwrap().is_none());
    }
}
