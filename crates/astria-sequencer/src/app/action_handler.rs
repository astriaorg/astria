use cnidarium::StateWrite;

#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    async fn check_stateless(&self) -> astria_eyre::eyre::Result<()>;

    async fn check_and_execute<S: StateWrite>(
        &self,
        mut state: S,
        context: crate::transaction::Context,
    ) -> astria_eyre::eyre::Result<()>;
}
