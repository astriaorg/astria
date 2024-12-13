use astria_core::{
    connect::{
        oracle::v2::{
            CurrencyPairState,
            QuotePrice,
        },
        types::v2::{
            CurrencyPairId,
            CurrencyPairNonce,
            Price,
        },
    },
    protocol::transaction::v1::action::AddCurrencyPairs,
    Timestamp,
};
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
    app::StateReadExt as _,
    connect::{
        market_map::state_ext::StateReadExt as _,
        oracle::state_ext::{
            StateReadExt as _,
            StateWriteExt,
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

        let next_currency_pair_id = state
            .get_next_currency_pair_id()
            .await
            .wrap_err("failed to get next currency pair id")?;
        let num_currency_pairs = state
            .get_num_currency_pairs()
            .await
            .wrap_err("failed to get number of currency pairs")?;
        let timestamp: tendermint_proto::google::protobuf::Timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get block timestamp")?
            .into();

        for pair in &self.pairs {
            let currency_pair_state = CurrencyPairState {
                price: QuotePrice {
                    price: Price::new(0),
                    block_timestamp: Timestamp {
                        seconds: timestamp.seconds,
                        nanos: timestamp.nanos,
                    },
                    block_height: state.get_block_height().await?,
                },
                nonce: CurrencyPairNonce::new(0),
                id: next_currency_pair_id,
            };
            state
                .put_currency_pair_state(pair.clone(), currency_pair_state)
                .wrap_err("failed to put currency pair state")?;
            num_currency_pairs
                .checked_add(1)
                .ok_or_eyre("overflow when incrementing number of currency pairs")?;
        }

        state
            .put_next_currency_pair_id(CurrencyPairId::new(num_currency_pairs))
            .wrap_err("failed to put next currency pair id")?;
        state
            .put_num_currency_pairs(
                num_currency_pairs.saturating_add(
                    self.pairs
                        .len()
                        .try_into()
                        .expect("number of pairs cannot exceed u64::MAX"),
                ),
            )
            .wrap_err("failed to put number of currency pairs")?;
        Ok(())
    }
}
