use async_trait::async_trait;
use astria_core::protocol::transaction::v1::action::UnenshrineAuctioneer;
use cnidarium::StateWrite;

use crate::action_handler::ActionHandler;

#[async_trait]
impl ActionHandler for UnenshrineAuctioneer {
    async fn check_stateless(&self) -> astria_eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> astria_eyre::Result<()> {
        Ok(())
    }
}
