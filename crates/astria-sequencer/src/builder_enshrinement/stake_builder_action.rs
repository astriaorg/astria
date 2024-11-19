use astria_core::protocol::transaction::v1::action::StakeBuilder;
use astria_eyre::eyre::Result;
use cnidarium::StateWrite;

use crate::app::ActionHandler;

#[async_trait::async_trait]
impl ActionHandler for StakeBuilder {
    async fn check_stateless(&self) -> Result<()> {
        // no need of any stateless checks
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        // check if address already exists as an enshrined account

        // check if the sender has enough amount to stake the collateral

        // create the enshrined sequencer account from the from address.

        // put it in the enshrined account list

        // transfer the collateral to the sequencer account

        Ok(())
    }
}
