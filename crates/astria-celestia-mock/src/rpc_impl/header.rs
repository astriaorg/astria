/// The Celestia JSON RPC header API.
///
/// This currently only provides a wrapper for the `header.NetworkHead` RPC method.
use celestia_client::celestia_types::ExtendedHeader;
use jsonrpsee::proc_macros::rpc;
// This only needs to be explicitly imported when activaing the server feature
// due to a quirk in the jsonrpsee proc macro.
use jsonrpsee::types::ErrorObjectOwned;

#[rpc(server)]
pub trait Header {
    #[method(name = "header.NetworkHead")]
    async fn header_network_head(&self) -> Result<ExtendedHeader, ErrorObjectOwned>;
}
