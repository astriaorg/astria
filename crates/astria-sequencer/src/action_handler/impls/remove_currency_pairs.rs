use astria_core::{
    protocol::transaction::v1::action::{
        RemoveCurrencyPairs,
    },
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
    action_handler::{
        ActionHandler,
    },
};

#[async_trait]
impl ActionHandler for RemoveCurrencyPairs {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        todo!();
        Ok(())
    }
}
