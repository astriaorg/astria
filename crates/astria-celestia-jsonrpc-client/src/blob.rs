use serde::Deserialize;

pub use crate::rpc_impl::blob::NAMESPACE_ID_AVAILABLE_LEN;
use crate::{
    rpc_impl::blob::{
        Blob as RawBlob,
        BlobClient as _,
        Namespace,
        NAMESPACE_VERSION_LEN,
        NAMESPACE_VERSION_ZERO_PREFIX_LEN,
    },
    Client,
    DeserializationError,
    Error,
};

/// A wrapper around the raw blob object submitted to the celestia JSON RPC.
///
/// See [`rpc_impl::blob::Blob`] for the whole thing.
#[derive(Debug, Default, Deserialize)]
#[serde(from = "RawBlob")]
pub struct Blob {
    pub namespace_id: [u8; NAMESPACE_ID_AVAILABLE_LEN],
    pub data: Vec<u8>,
}

impl From<RawBlob> for Blob {
    fn from(raw_blob: RawBlob) -> Self {
        Blob::from_raw_blob(raw_blob)
    }
}

impl Blob {
    fn from_raw_blob(raw_blob: RawBlob) -> Self {
        let RawBlob {
            namespace,
            data,
            ..
        } = raw_blob;
        let mut namespace_id = [0u8; NAMESPACE_ID_AVAILABLE_LEN];
        namespace_id.copy_from_slice(
            &(*namespace)[NAMESPACE_VERSION_LEN + NAMESPACE_VERSION_ZERO_PREFIX_LEN..],
        );
        Self {
            namespace_id,
            data,
        }
    }

    /// Construct a new blob with a zero namespace ID and
    /// empty data.
    ///
    /// Note that the default namespace ID (all zeros) falls in the
    /// reserved range and will result in an error from the JSON RPC
    /// endpoint. See [`Blob::namespace_id`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the namespace ID to the given value.
    ///
    /// At the moment the length of a celestia namespace is 10 bytes long.
    ///
    /// Note that this client performs no extra verification. If the provided namespace ID
    /// falls into the reserved region then the celestia JSON RPC service will likely return
    /// an error.
    ///
    /// As of rev 71e5006 the reserved namespace are all slices that match
    /// `[0, 0, 0, 0, 0, 0, 0, 0, 0, x]`, with x falling in the range from to 255.
    /// See also [`Namespace.IsReserved`] and [`namespace.MaxReservedNamespace`]
    ///
    /// [`Namespace.IsReserved`]: https://github.com/celestiaorg/celestia-app/blob/71e500611d51c9f6444748ff8655415eaae03356/pkg/namespace/namespace.go#L124
    /// [`namespace.MaxReservedNamespace`]: https://github.com/celestiaorg/celestia-app/blob/71e500611d51c9f6444748ff8655415eaae03356/pkg/namespace/consts.go#L56
    pub fn set_namespace_id(
        &mut self,
        namespace_id: [u8; NAMESPACE_ID_AVAILABLE_LEN],
    ) -> &mut Self {
        self.namespace_id = namespace_id;
        self
    }

    /// Sets the data in the blob to the provided value.
    pub fn set_data(&mut self, data: Vec<u8>) -> &mut Self {
        self.data = data;
        self
    }

    /// Overwrites the data in the blob with the bytes in the provided slice.
    pub fn set_data_from_slice(&mut self, data: &[u8]) -> &mut Self {
        self.data.clear();
        self.data.extend_from_slice(data);
        self
    }

    #[must_use]
    pub fn into_raw_blob(self) -> crate::rpc_impl::blob::Blob {
        use crate::rpc_impl::blob::Blob;
        let Self {
            namespace_id,
            data,
        } = self;
        Blob {
            namespace: Namespace::new_v0(namespace_id),
            data,
            ..Blob::default()
        }
    }
}

#[derive(Debug)]
pub struct GetAllRequest {
    pub height: u64,
    pub namespace_ids: Vec<[u8; NAMESPACE_ID_AVAILABLE_LEN]>,
}

#[derive(Debug)]
pub struct GetAllResponse {
    pub blobs: Vec<Blob>,
}

#[derive(Debug)]
pub struct SubmitRequest {
    pub blobs: Vec<Blob>,
}

#[derive(Debug)]
pub struct SubmitResponse {
    pub height: u64,
}

impl Client {
    /// Issue a `blob.GetAll` JSON RPC.
    ///
    /// # Errors
    /// Returns an error if the JSON RPC was not successful, or if the
    /// response could not be deserialized into a [`GetAllResponse`].
    pub async fn blob_get_all(&self, request: GetAllRequest) -> Result<GetAllResponse, Error> {
        const RPC_NAME: &str = "blob.GetAll";
        let GetAllRequest {
            height,
            namespace_ids,
        } = request;
        let namespaces = namespace_ids
            .into_iter()
            .map(Namespace::new_v0)
            .collect::<Vec<_>>();
        let raw_json = self
            .inner
            .get_all(height, &namespaces)
            .await
            .map_err(|e| Error::rpc(e, RPC_NAME))?;
        let blobs: Vec<Blob> = serde_json::from_str(raw_json.get())
            .map_err(|e| DeserializationError {
                source: e,
                rpc: RPC_NAME,
                deser_target: "GetAllResponse",
                raw_json: raw_json.clone(),
            })
            .map_err(|e| Error::deserialization(e, RPC_NAME))?;
        Ok(GetAllResponse {
            blobs,
        })
    }

    /// Call the `blob.Submit` RPC.
    ///
    /// This is a high level API for the celestia `blob.Submit` method.
    ///
    /// # Errors
    /// This has the same errors conditions as using the lower level
    /// [`BlobClient::submit`].
    pub async fn blob_submit(&self, request: SubmitRequest) -> Result<SubmitResponse, Error> {
        const RPC_NAME: &str = "blob.Submit";
        let raw_blobs: Vec<_> = request.blobs.into_iter().map(Blob::into_raw_blob).collect();
        let raw_json = self
            .inner
            .submit(&raw_blobs)
            .await
            .map_err(|e| Error::rpc(e, RPC_NAME))?;
        let height: u64 = serde_json::from_str(raw_json.get())
            .map_err(|e| DeserializationError {
                source: e,
                rpc: RPC_NAME,
                deser_target: "SubmitResponse",
                raw_json: raw_json.clone(),
            })
            .map_err(|e| Error::deserialization(e, RPC_NAME))?;
        let rsp = SubmitResponse {
            height,
        };
        Ok(rsp)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Blob,
        SubmitRequest,
    };
    use crate::{
        blob::GetAllRequest,
        test_utils::make_client,
    };

    #[tokio::test]
    #[serial_test::serial]
    #[ignore = "this needs to be run against a running celestia cluster"]
    async fn blob_submit_works() {
        let client = make_client().await;
        let req = SubmitRequest {
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
        let _rsp = client.blob_submit(req).await.unwrap();
    }

    #[tokio::test]
    #[serial_test::serial]
    #[ignore = "this needs to be run against a running celestia cluster"]
    async fn blob_get_all_works() {
        let data = b"helloworld";
        let client = make_client().await;
        // First submit a blob for inclusion
        let req = SubmitRequest {
            blobs: vec![Blob {
                namespace_id: *b"shrd-sqncr",
                data: data.to_vec(),
            }],
        };
        let rsp = client.blob_submit(req).await.unwrap();
        // Then retrieve it using `blob.GetAll`
        let req = GetAllRequest {
            height: rsp.height,
            namespace_ids: vec![*b"shrd-sqncr"],
        };
        let rsp = client.blob_get_all(req).await.unwrap();
        assert_eq!(1, rsp.blobs.len());
        let received_data = &*rsp.blobs[0].data;
        assert_eq!(data, received_data);
    }
}
