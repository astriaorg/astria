use cnidarium::StateWrite;
use astria_core::protocol::transaction::v1::action::UnstakeBuilder;
use crate::app::{ActionHandler, StateReadExt};

#[async_trait::async_trait]
impl ActionHandler for UnstakeBuilder {
    async fn check_stateless(&self) -> astria_eyre::Result<()> {
        todo!()
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> astria_eyre::Result<()> {
        // check if address exists as an enshrined account

        // if exists then add to unstaking list for 21 days
        todo!()
    }
}