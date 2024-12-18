use astria_core::protocol::transaction::v1::action::UpdateMarkets;
use astria_eyre::eyre::{
    self,
    bail,
    eyre,
    Context as _,
    OptionExt as _,
};
use cnidarium::StateWrite;

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
impl ActionHandler for UpdateMarkets {
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
        if !market_authorities.contains(&from) {
            bail!("address {from} is not a market authority");
        }

        // update existing markets, erroring if any do not exist in the current map
        let mut market_map = state
            .get_market_map()
            .await
            .wrap_err("failed to get market map")?
            .ok_or_eyre("market map not found in state")?;
        for market in &self.update_markets {
            let ticker_key = market.ticker.currency_pair.to_string();
            *market_map
                .markets
                .get_mut(&ticker_key)
                .ok_or_else(|| eyre!("market for ticker {ticker_key} not found in market map"))? =
                market.clone();
        }
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
            MarketMap,
            Params,
        },
        primitive::v1::TransactionId,
    };
    use indexmap::IndexMap;

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        app::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        test_utils::example_ticker,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

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

        let ticker_1 = example_ticker("ticker_1".to_string());
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

        let ticker = example_ticker("ticker".to_string());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let action = UpdateMarkets {
            update_markets: vec![market.clone()],
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

        state
            .put_market_map(MarketMap {
                markets: IndexMap::new(),
            })
            .unwrap();

        let ticker = example_ticker("ticker".to_string());
        let market = Market {
            ticker: ticker.clone(),
            provider_configs: vec![],
        };

        let action = UpdateMarkets {
            update_markets: vec![market.clone()],
        };

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

        let action = UpdateMarkets {
            update_markets: vec![],
        };

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map not found in state"));
    }

    #[tokio::test]
    async fn update_markets_fails_if_authority_is_invalid() {
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

        let action = UpdateMarkets {
            update_markets: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains(&format!(
            "address {authority_address} is not a market authority"
        )));
        assert!(state.get_market_map().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_markets_fails_if_params_not_found() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = UpdateMarkets {
            update_markets: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(
            res.to_string()
                .contains("market map params not found in state")
        );
        assert!(state.get_market_map().await.unwrap().is_none());
    }
}
