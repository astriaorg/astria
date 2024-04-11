use std::sync::Arc;

use astria_core::generated::{
    celestia::v1::query_client::QueryClient as BlobQueryClient,
    cosmos::{
        auth::v1beta1::query_client::QueryClient as AuthQueryClient,
        base::tendermint::v1beta1::{
            service_client::ServiceClient as NodeInfoClient,
            GetNodeInfoRequest,
        },
        tx::v1beta1::service_client::ServiceClient as TxClient,
    },
};
use http::Uri;
use tendermint::account::Id as AccountId;
use tonic::transport::{
    Channel,
    Endpoint,
};
use tracing::trace;

use super::{
    super::State,
    Bech32Address,
    Bech32EncodeError,
    CelestiaClient,
    CelestiaKeys,
    Error,
    GrpcResponseError,
};

/// A builder for a [`CelestiaClient`].
#[derive(Clone)]
pub(in crate::relayer) struct Builder {
    /// The inner `tonic` gRPC channel shared by the various generated gRPC clients.
    grpc_channel: Channel,
    /// The crypto keys associated with our Celestia account.
    signing_keys: CelestiaKeys,
    /// The Bech32-encoded address of our Celestia account.
    address: Bech32Address,
    /// A handle to the mutable state of the relayer.
    state: Arc<State>,
}

impl Builder {
    /// Returns a new `Builder`, or an error if Bech32-encoding the `signing_keys` address fails.
    pub(in crate::relayer) fn new(
        uri: Uri,
        signing_keys: CelestiaKeys,
        state: Arc<State>,
    ) -> Result<Self, Error> {
        let grpc_channel = Endpoint::from(uri).connect_lazy();
        let address = bech32_encode(&signing_keys.address)?;
        Ok(Self {
            grpc_channel,
            signing_keys,
            address,
            state,
        })
    }

    /// Returns a new `CelestiaClient` initialized with info retrieved from the Celestia app.
    ///
    /// On failure, the builder is returned along with the error in order to support retrying.
    pub(in crate::relayer) async fn try_build(self) -> Result<CelestiaClient, Error> {
        let chain_id = self.fetch_chain_id().await?;

        let Self {
            grpc_channel,
            signing_keys,
            address,
            state,
        } = self;

        let tx_client = TxClient::new(grpc_channel.clone());
        let blob_query_client = BlobQueryClient::new(grpc_channel.clone());
        let auth_query_client = AuthQueryClient::new(grpc_channel.clone());
        let mut client = CelestiaClient {
            grpc_channel,
            tx_client,
            blob_query_client,
            auth_query_client,
            signing_keys,
            address,
            chain_id,
        };

        client.fetch_and_cache_cost_params(state.clone()).await?;
        state.set_celestia_connected(true);
        Ok(client)
    }

    async fn fetch_chain_id(&self) -> Result<String, Error> {
        let mut node_info_client = NodeInfoClient::new(self.grpc_channel.clone());
        let response = node_info_client.get_node_info(GetNodeInfoRequest {}).await;
        trace!(?response);
        let chain_id = response
            .map_err(|status| Error::FailedToGetNodeInfo(GrpcResponseError::from(status)))?
            .into_inner()
            .default_node_info
            .ok_or_else(|| Error::EmptyNodeInfo)?
            .network;
        Ok(chain_id)
    }
}

fn bech32_encode(address: &AccountId) -> Result<Bech32Address, Error> {
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/app/app.go#L104
    const ACCOUNT_ADDRESS_PREFIX: bech32::Hrp = bech32::Hrp::parse_unchecked("celestia");

    let encoded_address =
        bech32::encode::<bech32::Bech32>(ACCOUNT_ADDRESS_PREFIX, address.as_bytes()).map_err(
            |error| Error::EncodeAddress {
                address: *address,
                source: Bech32EncodeError::from(error),
            },
        )?;
    Ok(Bech32Address(encoded_address))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_bech32_encode_known_address() {
        // These hard-coded values were generated using the `address.String` impl from
        // https://github.com/cosmos/cosmos-sdk/blob/v0.46.14/types/address.go#L297
        let account = AccountId::new([
            210, 116, 151, 227, 194, 250, 224, 24, 247, 10, 99, 245, 161, 33, 75, 209, 255, 243,
            153, 41,
        ]);
        let expected = "celestia16f6f0c7zltsp3ac2v066zg2t68ll8xffpq7dvr";
        let actual = bech32_encode(&account).expect("should encode");
        assert_eq!(actual.0, expected);
    }
}
