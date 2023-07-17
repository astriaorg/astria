use serde::Deserialize;

use crate::{
    rpc_impl::header::HeaderClient as _,
    Client,
    DeserializationError,
    Error,
};

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct Commit {
    pub height: u64,
    #[serde(flatten)]
    pub rest: serde_json::Value,
}

/// The response of the `header.NetworkHead` JSON RPC.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "server", derive(serde::Serialize))]
pub struct NetworkHeaderResponse {
    pub commit: Commit,
    #[serde(flatten)]
    pub inner: serde_json::Value,
}

impl NetworkHeaderResponse {
    /// Return the current height of the network as reported by the response.
    ///
    /// This function returns the `.commit.height` field of the response received from
    /// the `header.NetworkHead` RPC call.
    #[must_use]
    pub fn height(&self) -> u64 {
        self.commit.height
    }
}

impl Client {
    /// Issue a `header.NetworkHead` JSON RPC.
    ///
    /// # Errors
    /// Returns an error if the JSON RPC was not successful, or if the
    /// response could not be deserialized into a [`NetworkHeaderResponse`].
    pub async fn header_network_head(&self) -> Result<NetworkHeaderResponse, Error> {
        const RPC_NAME: &str = "header.NetworkHead";
        let raw_json = self
            .inner
            .network_head()
            .await
            .map_err(|e| Error::rpc(e, RPC_NAME))?;
        let rsp: NetworkHeaderResponse = serde_json::from_str(raw_json.get())
            .map_err(|e| DeserializationError {
                source: e,
                rpc: RPC_NAME,
                deser_target: "NetworkHeaderResponse",
                raw_json: raw_json.clone(),
            })
            .map_err(|e| Error::deserialization(e, RPC_NAME))?;
        Ok(rsp)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils;

    #[tokio::test]
    #[serial_test::parallel]
    #[ignore = "this needs to be run against a running celestia cluster"]
    async fn network_head_works() {
        let client = test_utils::make_client().await;
        let _rsp = client.header_network_head().await.unwrap();
    }
}
