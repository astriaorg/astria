use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

#[rpc(client)]
trait Share {
    #[method(name = "share.SharesAvailable")]
    async fn shares_available(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "share.ProbabilityOfAvailability")]
    async fn probability_of_availability(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "share.GetShare")]
    async fn get_share(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "share.GetEDS")]
    async fn get_eds(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "share.GetSharesByNamespace")]
    async fn get_shares_by_namespace(&self) -> Result<serde_json::Value, Error>;
}

