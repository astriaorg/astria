use astria_core::protocol::transaction::v1::action::RemoveCurrencyPairs;
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    action_handler::ActionHandler,
    oracles::price_feed::{
        market_map::state_ext::StateReadExt as _,
        oracle::state_ext::{
            StateReadExt as _,
            StateWriteExt,
        },
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for RemoveCurrencyPairs {
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

        let mut num_currency_pairs = state
            .get_num_currency_pairs()
            .await
            .wrap_err("failed to get number of currency pairs")?;
        ensure!(
            num_currency_pairs >= self.pairs.len() as u64,
            "cannot remove more currency pairs than exist",
        );

        for pair in &self.pairs {
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
            .wrap_err("failed to put number of currency pairs")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use astria_core::{
        oracles::price_feed::{
            market_map::v2::Params,
            oracle::v2::CurrencyPairState,
            types::v2::{
                CurrencyPair,
                CurrencyPairId,
                CurrencyPairNonce,
            },
        },
        primitive::v1::TransactionId,
        protocol::transaction::v1::action::RemoveCurrencyPairs,
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        app::test_utils::get_alice_signing_key,
        benchmark_and_test_utils::astria_address,
        oracles::price_feed::{
            market_map::state_ext::StateWriteExt as _,
            oracle::state_ext::StateWriteExt as _,
        },
        transaction::{
            StateWriteExt,
            TransactionContext,
        },
    };

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

        let action = RemoveCurrencyPairs {
            pairs: pairs.clone(),
        };
        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(state.get_num_currency_pairs().await.unwrap(), 1);
    }
}
