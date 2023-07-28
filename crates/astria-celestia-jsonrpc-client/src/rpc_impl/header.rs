/// The Celestia JSON RPC header API.
///
/// This currently only provides a wrapper for the `header.Network` RPC method.
/// It is not completely clear what value `fee` should take. According to the
use jsonrpsee::proc_macros::rpc;
// This only needs to be explicitly imported when activaing the server feature
// due to a quirk in the jsonrpsee proc macro.
#[cfg(feature = "server")]
use jsonrpsee::types::ErrorObjectOwned;

#[cfg_attr(not(feature = "server"), rpc(client))]
#[cfg_attr(feature = "server", rpc(client, server))]
pub trait Header {
    #[method(name = "header.NetworkHead")]
    async fn network_head(&self) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned>;
}
