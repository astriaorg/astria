use std::sync::Arc;

use astria_core::generated::cosmos::{
    base::tendermint::v1beta1::{
        service_client::ServiceClient as NodeInfoClient,
        GetNodeInfoRequest,
    },
    tx::v1beta1::service_client::ServiceClient as TxClient,
};
use http::Uri;
use tendermint::account::Id as AccountId;
use thiserror::Error;
use tonic::transport::{
    Channel,
    Endpoint,
};
use tracing::{
    info,
    trace,
};

use super::{
    super::State,
    Bech32Address,
    CelestiaClient,
    CelestiaKeys,
    GrpcResponseError,
};

/// An error when building the `CelestiaClient`.
#[derive(Error, Clone, Debug)]
#[non_exhaustive]
pub(in crate::relayer) enum BuilderError {
    /// Failed to Bech32-encode our Celestia address.
    #[error("failed to Bech32-encode our celestia address {address}")]
    EncodeAddress {
        address: AccountId,
        source: Bech32EncodeError,
    },
    /// The celestia app responded with the given error status to a `GetNodeInfoRequest`.
    #[error("failed to get celestia node info")]
    FailedToGetNodeInfo(#[source] GrpcResponseError),
    /// The node info response was empty.
    #[error("the celestia node info response was empty")]
    EmptyNodeInfo,
    /// Mismatch in Celestia chain ID.
    #[error(
        "mismatch in celestia chain id, configured id: `{configured}`, received id: `{received}`"
    )]
    MismatchedCelestiaChainId {
        configured: String,
        received: String,
    },
}

/// An error while encoding a Bech32 string.
#[derive(Error, Clone, Debug)]
#[error(transparent)]
pub(in crate::relayer) struct Bech32EncodeError(#[from] bech32::EncodeError);

/// A builder for a [`CelestiaClient`].
#[derive(Clone)]
pub(in crate::relayer) struct Builder {
    configured_celestia_chain_id: String,
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
        configured_celestia_chain_id: String,
        uri: Uri,
        signing_keys: CelestiaKeys,
        state: Arc<State>,
    ) -> Result<Self, BuilderError> {
        let grpc_channel = Endpoint::from(uri).connect_lazy();
        let address = bech32_encode(&signing_keys.address)?;
        Ok(Self {
            configured_celestia_chain_id,
            grpc_channel,
            signing_keys,
            address,
            state,
        })
    }

    /// Returns a new `CelestiaClient` initialized with info retrieved from the Celestia app.
    pub(in crate::relayer) async fn try_build(self) -> Result<CelestiaClient, BuilderError> {
        let reeceived_celestia_chain_id = self.fetch_celestia_chain_id().await?;

        let Self {
            configured_celestia_chain_id,
            grpc_channel,
            signing_keys,
            address,
            state,
        } = self;

        if reeceived_celestia_chain_id != configured_celestia_chain_id {
            return Err(BuilderError::MismatchedCelestiaChainId {
                configured: configured_celestia_chain_id,
                received: reeceived_celestia_chain_id,
            });
        }

        info!(celestia_chain_id = %reeceived_celestia_chain_id, "confirmed celestia chain id");
        state.set_celestia_connected(true);

        let tx_client = TxClient::new(grpc_channel.clone());
        Ok(CelestiaClient {
            grpc_channel,
            tx_client,
            signing_keys,
            address,
            chain_id: reeceived_celestia_chain_id,
        })
    }

    async fn fetch_celestia_chain_id(&self) -> Result<String, BuilderError> {
        let mut node_info_client = NodeInfoClient::new(self.grpc_channel.clone());
        let response = node_info_client.get_node_info(GetNodeInfoRequest {}).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        let chain_id = response
            .map_err(|status| BuilderError::FailedToGetNodeInfo(GrpcResponseError::from(status)))?
            .into_inner()
            .default_node_info
            .ok_or(BuilderError::EmptyNodeInfo)?
            .network;
        Ok(chain_id)
    }
}

fn bech32_encode(address: &AccountId) -> Result<Bech32Address, BuilderError> {
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/app/app.go#L104
    const ACCOUNT_ADDRESS_PREFIX: bech32::Hrp = bech32::Hrp::parse_unchecked("celestia");

    let encoded_address =
        bech32::encode::<bech32::Bech32>(ACCOUNT_ADDRESS_PREFIX, address.as_bytes()).map_err(
            |error| BuilderError::EncodeAddress {
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
