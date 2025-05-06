use astria_core::{
    oracles::price_feed::{
        oracle::v2::CurrencyPairState,
        types::v2::{
            CurrencyPair,
            CurrencyPairNonce,
        },
    },
    protocol::transaction::v1::action::CurrencyPairsChange,
};
use astria_eyre::eyre::{
    ensure,
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
    authority::StateReadExt as _,
    oracles::price_feed::oracle::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for CurrencyPairsChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        match self {
            CurrencyPairsChange::Addition(currency_pairs) => {
                check_and_execute_currency_pairs_addition(state, currency_pairs).await
            }
            CurrencyPairsChange::Removal(currency_pairs) => {
                check_and_execute_currency_pairs_removal(state, currency_pairs).await
            }
        }
    }
}

async fn check_and_execute_currency_pairs_addition<S: StateWrite>(
    mut state: S,
    currency_pairs: &[CurrencyPair],
) -> Result<()> {
    validate_signer_is_sudo(&state).await?;

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
    validate_signer_is_sudo(&state).await?;

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

async fn validate_signer_is_sudo<S: StateRead>(state: S) -> Result<()> {
    let from = state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action")
        .address_bytes();
    let sudo_address = state
        .get_sudo_address()
        .await
        .wrap_err("failed to get sudo address from state")?;
    ensure!(sudo_address == from, "signer is not the sudo key");
    Ok(())
}

#[cfg(test)]
mod test {
    use astria_core::{
        oracles::price_feed::{
            oracle::v2::CurrencyPairState,
            types::v2::CurrencyPairId,
        },
        primitive::v1::TransactionId,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        app::test_utils::get_alice_signing_key,
        authority::StateWriteExt as _,
        benchmark_and_test_utils::astria_address,
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

        state.put_sudo_address(alice_address).unwrap();
        state
            .put_next_currency_pair_id(CurrencyPairId::new(0))
            .unwrap();
        state.put_num_currency_pairs(0).unwrap();

        let pairs = vec![
            "BTC/USD".parse().unwrap(),
            "ETH/USD".parse().unwrap(),
            "BTC/USD".parse().unwrap(),
        ];
        let action = CurrencyPairsChange::Addition(pairs.clone());
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

        state.put_sudo_address(alice_address).unwrap();
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

        let action = CurrencyPairsChange::Removal(pairs.clone());
        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(state.get_num_currency_pairs().await.unwrap(), 1);
    }
}
