use astria_core::primitive::v1::{
    asset,
    TransactionId,
};
use cnidarium::StateWrite;

#[async_trait::async_trait]
pub(crate) trait FeeHandler {
    async fn calculate_and_pay_fees<S: StateWrite>(
        &self,
        mut state: S,
    ) -> astria_eyre::eyre::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    pub(crate) asset: asset::Denom,
    pub(crate) amount: u128,
    pub(crate) source_transaction_id: TransactionId,
    pub(crate) source_action_index: u64,
}
