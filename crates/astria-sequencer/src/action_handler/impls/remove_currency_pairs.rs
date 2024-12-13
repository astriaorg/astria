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

        let num_currency_pairs = state
            .get_num_currency_pairs()
            .await
            .wrap_err("failed to get number of currency pairs")?;
        ensure!(
            num_currency_pairs >= self.pairs.len() as u64,
            "cannot remove more currency pairs than exist",
        );

        for pair in &self.pairs {
            state
                .delete_currency_pair(pair)
                .await
                .wrap_err("failed to delete currency pair")?;
            num_currency_pairs
                .checked_sub(1)
                .ok_or_eyre("failed to decrement number of currency pairs")?;
        }

        state
            .put_num_currency_pairs(num_currency_pairs)
            .wrap_err("failed to put number of currency pairs")?;

        Ok(())
    }
}
