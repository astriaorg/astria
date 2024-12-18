use astria_core::{
    connect::{
        oracle::v2::CurrencyPairState,
        types::v2::CurrencyPairNonce,
    },
    protocol::transaction::v1::action::AddCurrencyPairs,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::debug;

use crate::{
    action_handler::ActionHandler,
    connect::{
        market_map::state_ext::StateReadExt as _,
        oracle::state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for AddCurrencyPairs {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
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

        let mut next_currency_pair_id = state
            .get_next_currency_pair_id()
            .await
            .wrap_err("failed to get next currency pair id")?;
        let mut num_currency_pairs = state
            .get_num_currency_pairs()
            .await
            .wrap_err("failed to get number of currency pairs")?;

        for pair in &self.pairs {
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
            .wrap_err("failed to put number of currency pairs")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        connect::{
            market_map::v2::Params,
            oracle::v2::CurrencyPairState,
            types::v2::CurrencyPairId,
        },
        primitive::v1::TransactionId,
        protocol::transaction::v1::action::AddCurrencyPairs,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        app::test_utils::get_alice_signing_key,
        benchmark_and_test_utils::astria_address,
        connect::market_map::state_ext::StateWriteExt as _,
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
        let action = AddCurrencyPairs {
            pairs: pairs.clone(),
        };
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
}
