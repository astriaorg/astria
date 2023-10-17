/// The Celestia JSON RPC blob API.
///
/// This currently only provides a wrapper for the `blob.Submit` RPC method.
use celestia_client::celestia_types::{
    blob::SubmitOptions,
    Blob,
};
use jsonrpsee::proc_macros::rpc;
// This only needs to be explicitly imported when activaing the server feature
// due to a quirk in the jsonrpsee proc macro.
use jsonrpsee::types::ErrorObjectOwned;

#[rpc(server)]
pub trait Blob {
    #[method(name = "blob.Submit")]
    async fn blob_submit(
        &self,
        blobs: Vec<Blob>,
        opts: SubmitOptions,
    ) -> Result<u64, ErrorObjectOwned>;
}
