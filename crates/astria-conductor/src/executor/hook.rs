#[async_trait::async_trait]
pub(crate) trait PreExecutionHook: Send + Sync {
    async fn populate_rollup_transactions(
        &mut self,
        sequenced_transactions: Vec<Vec<u8>>,
    ) -> super::eyre::Result<Vec<Vec<u8>>>;
}
