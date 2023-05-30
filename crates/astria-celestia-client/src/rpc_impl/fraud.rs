use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

use crate::serde::Base64Standard;

#[rpc(client)]
trait Fraud {
    #[method(name = "fraud.Get")]
    async fn get(&self, proof_type: String) -> Result<Vec<Proof>, Error>;

    // async fn subscribe(&self) -> Result<serde_json::Value, Error>;
    // #[subscription(name = "fraud.Subscribe", item = "String")]
    #[subscription(name = "fraud.Subscribe", item = "String")]
    async fn subscribe(&self, proof_type: String) -> Result<Subscription<String>, Error>;
}

#[derive(Deserialize, Debug)]
pub struct Proof {
    proof_type: String,
    #[serde(with = "Base64Standard")]
    data: Vec<u8>,
}
