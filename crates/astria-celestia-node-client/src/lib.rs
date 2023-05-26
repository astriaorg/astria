//! Rust interface to the Celestia Node API using JsonRPC

// Note to developers: the `Error` type in the trait definitions that
// get passed to the `rpc` proc macro is a work around until jsonrpsee
// fully implements custom errors. See"
// https://github.com/paritytech/jsonrpsee/issues/1136
// https://github.com/paritytech/jsonrpsee/issues/1067

use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SubmitForPayBlobResponse {
    pub height: u64,
    pub txhash: String,
    pub data: String,
    pub raw_log: serde_json::Value,
    pub logs: serde_json::Value,
    pub gas_wanted: u64,
    pub gas_used: u64,
    pub events: serde_json::Value,
}

#[rpc(client)]
trait State {
    #[method(name = "state.SubmitPayForBlob")]
    async fn submit_pay_for_blob(
        &self,
        namespace: String,
        data: String,
        fee: String,
        gas_limit: u64,
    ) -> Result<SubmitForPayBlobResponse, Error>;
}

#[rpc(client)]
trait Header {
    #[method(name = "header.GetByHeight")]
    async fn get_by_height(&self, height: u64) -> Result<serde_json::Value, Error>;
}
