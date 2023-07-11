use serde::Deserialize;

use crate::{
    blob::Blob,
    rpc_impl::state::StateClient as _,
    Client,
    DeserializationError,
    Error,
};

#[derive(Debug)]
pub struct SubmitPayForBlobRequest {
    pub fee: u128,
    pub gas_limit: u64,
    pub blobs: Vec<Blob>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitPayForBlobResponse {
    pub height: u64,
    #[serde(flatten)]
    pub rest: serde_json::Value,
}

impl Client {
    /// Issue a `state.SubmitPayForBlob` JSON RPC.
    ///
    /// This is a high level API for the celestia `blob.Submit` method.
    ///
    /// # Errors
    /// This has the same errors conditions as using the lower level
    /// [`StateClient::submit_pay_for_blob`].
    pub async fn state_submit_pay_for_blob(
        &self,
        request: SubmitPayForBlobRequest,
    ) -> Result<SubmitPayForBlobResponse, Error> {
        use crate::rpc_impl::state::Fee;
        const RPC_NAME: &str = "state.SubmitPayForBlob";

        let SubmitPayForBlobRequest {
            fee,
            gas_limit,
            blobs,
        } = request;
        let raw_blobs: Vec<_> = blobs.into_iter().map(Blob::into_raw_blob).collect();
        let raw_json = self
            .inner
            .submit_pay_for_blob(Fee::from_u128(fee), gas_limit, &raw_blobs)
            .await
            .map_err(|e| Error::rpc(e, RPC_NAME))?;
        let rsp: SubmitPayForBlobResponse = serde_json::from_str(raw_json.get())
            .map_err(|e| DeserializationError {
                source: e,
                rpc: RPC_NAME,
                deser_target: "SubmitPayForBlobResponse",
                raw_json: raw_json.clone(),
            })
            .map_err(|e| Error::deserialization(e, RPC_NAME))?;
        Ok(rsp)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Blob,
        SubmitPayForBlobRequest,
    };
    use crate::test_utils::make_client;
    #[tokio::test]
    #[serial_test::serial]
    #[ignore = "this needs to be run against a running celestia cluster"]
    async fn submit_pay_for_blob_works() {
        let client = make_client().await;
        let req = SubmitPayForBlobRequest {
            fee: 10_000,
            gas_limit: 100_000,
            blobs: vec![
                Blob {
                    namespace_id: *b"shrd-sqncr",
                    data: b"helloworld".to_vec(),
                },
                Blob {
                    namespace_id: *b"shrd-sqncr",
                    data: b"helloworld".to_vec(),
                },
            ],
        };
        let _rsp = client.state_submit_pay_for_blob(req).await.unwrap();
    }
}
