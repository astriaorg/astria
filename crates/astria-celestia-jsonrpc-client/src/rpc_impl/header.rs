/// The Celestia JSON RPC header API.
///
/// This currently only provides a wrapper for the `header.Network` RPC method.
/// It is not completely clear what value `fee` should take. According to the
use jsonrpsee::proc_macros::rpc;

#[rpc(client)]
pub trait Header {
    #[method(name = "header.NetworkHead")]
    async fn network_head(&self) -> Result<Box<serde_json::value::RawValue>, Error>;
}
