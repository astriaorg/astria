use jsonrpsee::proc_macros::rpc;

#[rpc(client)]
trait State {
    #[method(name = "state.IsStopped")]
    async fn is_stopped(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.AccountAddress")]
    async fn account_address(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.Balance")]
    async fn balance(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.BalanceForAddress")]
    async fn balance_for_address(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.Transfer")]
    async fn transfer(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.SubmitTx")]
    async fn submit_tx(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.SubmitPayForBlob")]
    async fn submit_pay_for_blob(
        &self,
        fee: String,
        gas_limit: u64,
        blobs: &[crate::Blob],
    ) -> Result<crate::SubmitPayForBlobResponse, Error>;

    #[method(name = "state.CancelUnbondingDelegation")]
    async fn cancel_unbonding_delegation(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.BeginRedelegate")]
    async fn begin_redelegate(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.Undelegate")]
    async fn undelegate(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.Delegate")]
    async fn delegate(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.QueryDelegation")]
    async fn query_delegation(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.QueryUnbonding")]
    async fn query_unbonding(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.QueryRedelegations")]
    async fn query_redelegations(&self) -> Result<serde_json::Value, Error>;
}
