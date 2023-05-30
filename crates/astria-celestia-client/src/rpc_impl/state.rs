use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

#[rpc(client)]
trait State {
    // #[method(name = "state.IsStopped")]
    // async fn is_stopped(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.AccountAddress")]
    // async fn account_address(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.Balance")]
    // async fn balance(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.BalanceForAddress")]
    // async fn balance_for_address(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.Transfer")]
    // async fn transfer(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.SubmitTx")]
    // async fn submit_tx(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "state.SubmitPayForBlob")]
    async fn submit_pay_for_blob(
        &self,
        namespace: String,
        data: String,
        fee: String,
        gas_limit: u64,
    ) -> Result<SubmitPayForBlobResponse, Error>;

    // #[method(name = "state.CancelUnbondingDelegation")]
    // async fn cancel_unbonding_delegation(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.BeginRedelegate")]
    // async fn begin_redelegate(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.Undelegate")]
    // async fn undelegate(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.Delegate")]
    // async fn delegate(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.QueryDelegation")]
    // async fn query_delegation(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.QueryUnbonding")]
    // async fn query_unbonding(&self) -> Result<serde_json::Value, Error>;

    // #[method(name = "state.QueryRedelegations")]
    // async fn query_redelegations(&self) -> Result<serde_json::Value, Error>;
}

#[derive(Deserialize, Debug)]
pub struct SubmitPayForBlobResponse {
    pub height: u64,
    pub txhash: String,
    pub data: String,
    pub raw_log: serde_json::Value,
    pub logs: serde_json::Value,
    pub gas_wanted: u64,
    pub gas_used: u64,
    pub events: serde_json::Value,
}
