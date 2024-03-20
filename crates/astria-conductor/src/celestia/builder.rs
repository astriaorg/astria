//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use celestia_client::celestia_types::nmt::Namespace;
use sequencer_client::HttpClient;
use tokio::sync::oneshot;

use super::Reader;
use crate::{
    celestia::block_verifier::BlockVerifier,
    executor,
};

pub(crate) struct Builder {
    pub(crate) celestia_http_endpoint: String,
    pub(crate) celestia_websocket_endpoint: String,
    pub(crate) celestia_token: String,
    pub(crate) executor: executor::Handle,
    pub(crate) sequencer_cometbft_client: HttpClient,
    pub(crate) sequencer_namespace: Namespace,
    pub(crate) shutdown: oneshot::Receiver<()>,
}

impl Builder {
    /// Creates a new [`Reader`] instance,
    pub(crate) fn build(self) -> Reader {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_cometbft_client,
            sequencer_namespace,
            shutdown,
        } = self;

        let block_verifier = BlockVerifier::new(sequencer_cometbft_client);

        Reader {
            executor,
            celestia_http_endpoint,
            celestia_ws_endpoint: celestia_websocket_endpoint,
            celestia_auth_token: celestia_token,
            block_verifier,
            sequencer_namespace,
            shutdown,
        }
    }
}
