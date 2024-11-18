use cnidarium::StateWrite;
use astria_core::protocol::transaction::v1::action::WithdrawBuilderCollateral;
use crate::app::ActionHandler;

impl ActionHandler for WithdrawBuilderCollateral {
    async fn check_stateless(&self) -> astria_eyre::Result<()> {
        // no need of any stateless checks
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> astria_eyre::Result<()> {
        // check if address has an existing unstaking entry

        // if the timestamp in the unstaking entry has crossed 21 days then
        // transfer the collateral value to the from address

        Ok(())
    }
}