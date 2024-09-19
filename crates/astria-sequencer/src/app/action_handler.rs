use cnidarium::StateWrite;

use crate::app::fee_handler::FeeHandler;

#[async_trait::async_trait]
pub(crate) trait ActionHandler: FeeHandler {
    // Commenting out for the time being as this is currentl nonot being used. Leaving this in
    // for reference as this is copied from cnidarium_component.
    // ```
    // type CheckStatelessContext: Clone + Send + Sync + 'static;
    // async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;
    // async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
    //     Ok(())
    // }
    // ```

    async fn check_stateless(&self) -> astria_eyre::eyre::Result<()>;

    async fn check_execute_and_pay_fees<S: StateWrite>(
        &self,
        mut state: S,
    ) -> astria_eyre::eyre::Result<()> {
        self.check_and_execute(&mut state).await?;
        self.calculate_and_pay_fees(&mut state).await?;
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S)
    -> astria_eyre::eyre::Result<()>;
}
