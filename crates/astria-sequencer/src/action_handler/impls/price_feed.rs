use astria_core::{
    oracles::price_feed::{
        oracle::v2::CurrencyPairState,
        types::v2::{
            CurrencyPair,
            CurrencyPairNonce,
        },
    },
    protocol::transaction::v1::action::{
        ChangeMarkets,
        CurrencyPairsChange,
        MarketMapChange,
        PriceFeed,
        UpdateMarketMapParams,
    },
};
use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::debug;

use crate::{
    action_handler::ActionHandler,
    address::StateReadExt as _,
    app::StateReadExt as _,
    authority::StateReadExt as _,
    oracles::price_feed::{
        market_map::state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        oracle::state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for PriceFeed {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        match self {
            PriceFeed::Oracle(CurrencyPairsChange::Addition(currency_pairs)) => {
                check_and_execute_currency_pairs_addition(state, currency_pairs).await
            }
            PriceFeed::Oracle(CurrencyPairsChange::Removal(currency_pairs)) => {
                check_and_execute_currency_pairs_removal(state, currency_pairs).await
            }
            PriceFeed::MarketMap(MarketMapChange::Markets(change_markets_action)) => {
                check_and_execute_change_markets(state, change_markets_action).await
            }
            PriceFeed::MarketMap(MarketMapChange::Params(update_market_map_params)) => {
                check_and_execute_update_market_map_params(state, update_market_map_params).await
            }
        }
    }
}

async fn check_and_execute_currency_pairs_addition<S: StateWrite>(
    mut state: S,
    currency_pairs: &[CurrencyPair],
) -> Result<()> {
    validate_signer_is_admin(&state).await?;

    let mut next_currency_pair_id = state
        .get_next_currency_pair_id()
        .await
        .wrap_err("failed to get next currency pair id")?;
    let mut num_currency_pairs = state
        .get_num_currency_pairs()
        .await
        .wrap_err("failed to get number of currency pairs")?;

    for pair in currency_pairs {
        if state
            .get_currency_pair_state(pair)
            .await
            .wrap_err("failed to get currency pair state")?
            .is_some()
        {
            debug!("currency pair {} already exists, skipping", pair);
            continue;
        }

        let currency_pair_state = CurrencyPairState {
            price: None,
            nonce: CurrencyPairNonce::new(0),
            id: next_currency_pair_id,
        };
        state
            .put_currency_pair_state(pair.clone(), currency_pair_state)
            .wrap_err("failed to put currency pair state")?;
        num_currency_pairs = num_currency_pairs
            .checked_add(1)
            .ok_or_eyre("overflow when incrementing number of currency pairs")?;
        next_currency_pair_id = next_currency_pair_id
            .increment()
            .ok_or_eyre("overflow when incrementing next currency pair id")?;
    }

    state
        .put_next_currency_pair_id(next_currency_pair_id)
        .wrap_err("failed to put next currency pair id")?;
    state
        .put_num_currency_pairs(num_currency_pairs)
        .wrap_err("failed to put number of currency pairs")
}

async fn check_and_execute_currency_pairs_removal<S: StateWrite>(
    mut state: S,
    currency_pairs: &[CurrencyPair],
) -> Result<()> {
    validate_signer_is_admin(&state).await?;

    let mut num_currency_pairs = state
        .get_num_currency_pairs()
        .await
        .wrap_err("failed to get number of currency pairs")?;
    ensure!(
        num_currency_pairs >= currency_pairs.len() as u64,
        "cannot remove more currency pairs than exist",
    );

    for pair in currency_pairs {
        if state
            .remove_currency_pair(pair)
            .await
            .wrap_err("failed to delete currency pair")?
        {
            num_currency_pairs = num_currency_pairs
                .checked_sub(1)
                .ok_or_eyre("failed to decrement number of currency pairs")?;
        }
    }

    state
        .put_num_currency_pairs(num_currency_pairs)
        .wrap_err("failed to put number of currency pairs")
}

async fn check_and_execute_change_markets<S: StateWrite>(
    mut state: S,
    change_markets_action: &ChangeMarkets,
) -> Result<()> {
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

    let mut market_map = state
        .get_market_map()
        .await
        .wrap_err("failed to get market map")?
        .ok_or_eyre("market map not found in state")?;
    match change_markets_action {
        ChangeMarkets::Create(create_markets) => {
            for market in create_markets {
                let ticker_key = market.ticker.currency_pair.to_string();
                if market_map.markets.contains_key(&ticker_key) {
                    bail!("market for ticker {ticker_key} already exists");
                }
                market_map.markets.insert(ticker_key, market.clone());
            }
        }
        ChangeMarkets::Update(update_markets) => {
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
        ChangeMarkets::Remove(remove_markets) => {
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

async fn check_and_execute_update_market_map_params<S: StateWrite>(
    mut state: S,
    update_market_map_params: &UpdateMarketMapParams,
) -> Result<()> {
    let from = &state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action")
        .address_bytes();
    let sudo_address = state
        .get_sudo_address()
        .await
        .wrap_err("failed to get sudo address from state")?;
    ensure!(sudo_address == *from, "signer is not the sudo key");
    state
        .put_params(update_market_map_params.params.clone())
        .wrap_err("failed to put params into state")?;
    Ok(())
}

async fn validate_signer_is_admin<S: StateRead>(state: S) -> Result<()> {
    // TODO: should we use the market map admin here, or a different admin?
    let admin = state
        .get_params()
        .await?
        .ok_or_eyre("market map params not set")?
        .admin;
    let from = state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action")
        .address_bytes();
    ensure!(
        from == admin.bytes(),
        "only the market map admin can add currency pairs"
    );
    Ok(())
}

#[cfg(test)]
mod test {
    use astria_core::{
        oracles::price_feed::{
            market_map::v2::{
                Market,
                MarketMap,
                Params,
            },
            oracle::v2::CurrencyPairState,
            types::v2::CurrencyPairId,
        },
        primitive::v1::TransactionId,
        protocol::transaction::v1::action::PriceFeed,
    };
    use cnidarium::StateDelta;
    use indexmap::IndexMap;

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        app::{
            test_utils::get_alice_signing_key,
            StateWriteExt as _,
        },
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
    async fn add_currency_pairs_with_duplicate() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        state.put_transaction_context(TransactionContext {
            address_bytes: alice.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        state
            .put_params(Params {
                market_authorities: vec![],
                admin: alice_address,
            })
            .unwrap();
        state
            .put_next_currency_pair_id(CurrencyPairId::new(0))
            .unwrap();
        state.put_num_currency_pairs(0).unwrap();

        let pairs = vec![
            "BTC/USD".parse().unwrap(),
            "ETH/USD".parse().unwrap(),
            "BTC/USD".parse().unwrap(),
        ];
        let action = PriceFeed::Oracle(CurrencyPairsChange::Addition(pairs.clone()));
        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(
            state
                .get_currency_pair_state(&pairs[0])
                .await
                .unwrap()
                .unwrap(),
            CurrencyPairState {
                price: None,
                nonce: CurrencyPairNonce::new(0),
                id: CurrencyPairId::new(0),
            }
        );
        assert_eq!(
            state
                .get_currency_pair_state(&pairs[1])
                .await
                .unwrap()
                .unwrap(),
            CurrencyPairState {
                price: None,
                nonce: CurrencyPairNonce::new(0),
                id: CurrencyPairId::new(1),
            }
        );
        assert_eq!(
            state.get_next_currency_pair_id().await.unwrap(),
            CurrencyPairId::new(2)
        );
        assert_eq!(state.get_num_currency_pairs().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn remove_currency_pairs_with_duplicate() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        state.put_transaction_context(TransactionContext {
            address_bytes: alice.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let pairs: Vec<CurrencyPair> = vec![
            "BTC/USD".parse().unwrap(),
            "ETH/USD".parse().unwrap(),
            "BTC/USD".parse().unwrap(),
        ];

        state
            .put_params(Params {
                market_authorities: vec![],
                admin: alice_address,
            })
            .unwrap();
        state.put_num_currency_pairs(3).unwrap();
        state
            .put_currency_pair_state(
                pairs[0].clone(),
                CurrencyPairState {
                    price: None,
                    nonce: CurrencyPairNonce::new(0),
                    id: CurrencyPairId::new(0),
                },
            )
            .unwrap();
        state
            .put_currency_pair_state(
                pairs[1].clone(),
                CurrencyPairState {
                    price: None,
                    nonce: CurrencyPairNonce::new(0),
                    id: CurrencyPairId::new(1),
                },
            )
            .unwrap();
        state
            .put_currency_pair_state(
                "TIA/USD".parse().unwrap(),
                CurrencyPairState {
                    price: None,
                    nonce: CurrencyPairNonce::new(0),
                    id: CurrencyPairId::new(2),
                },
            )
            .unwrap();

        let action = PriceFeed::Oracle(CurrencyPairsChange::Removal(pairs.clone()));
        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(state.get_num_currency_pairs().await.unwrap(), 1);
    }

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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Create(vec![
            market.clone(),
        ])));
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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Create(vec![])));
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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Create(vec![])));
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res
            .to_string()
            .contains("market map params not found in state"));
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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Remove(vec![
            Market {
                ticker,
                provider_configs: vec![],
            },
        ])));
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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Remove(vec![
            Market {
                ticker: example_ticker_from_currency_pair("DIFBASE", "DIFQUOTE", String::new()),
                provider_configs: vec![],
            },
        ])));
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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Update(vec![
            market.clone(),
        ])));

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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Update(vec![
            market.clone(),
        ])));

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

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Update(vec![])));

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map not found in state"));
    }

    #[tokio::test]
    async fn update_markets_fails_if_market_map_is_empty() {
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
        state
            .put_market_map(MarketMap {
                markets: IndexMap::new(),
            })
            .unwrap();

        let params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        state.put_params(params).unwrap();

        let action = PriceFeed::MarketMap(MarketMapChange::Markets(ChangeMarkets::Update(vec![])));

        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("market map is empty"));
    }

    #[tokio::test]
    async fn update_market_map_params_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);

        state.put_sudo_address(authority_address).unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        assert!(state.get_params().await.unwrap().is_none());

        let expected_params = Params {
            market_authorities: vec![authority_address],
            admin: authority_address,
        };
        let action = PriceFeed::MarketMap(MarketMapChange::Params(UpdateMarketMapParams {
            params: expected_params.clone(),
        }));
        action.check_and_execute(&mut state).await.unwrap();
        let actual_params = state
            .get_params()
            .await
            .unwrap()
            .expect("params should be present");
        assert_eq!(actual_params, expected_params);
    }

    #[tokio::test]
    async fn update_market_map_params_fails_if_signer_is_invalid() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        let invalid_address = astria_address(&[1; 20]);

        state.put_sudo_address(authority_address).unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *invalid_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = PriceFeed::MarketMap(MarketMapChange::Params(UpdateMarketMapParams {
            params: Params {
                market_authorities: vec![invalid_address],
                admin: invalid_address,
            },
        }));
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("signer is not the sudo key"));
    }
}
