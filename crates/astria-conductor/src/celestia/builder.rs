//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use std::time::Duration;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::{
    celestia_types::nmt::Namespace,
    jsonrpsee::http_client::HttpClient as CelestiaClient,
};
use sequencer_client::HttpClient as SequencerClient;
use tokio_util::sync::CancellationToken;

use super::{
    block_verifier::BlockVerifier,
    latest_height_stream::stream_latest_heights,
    Reader,
};
use crate::executor;

pub(crate) struct Builder {
    pub(crate) celestia_block_time: Duration,
    pub(crate) celestia_http_endpoint: String,
    pub(crate) celestia_token: String,
    pub(crate) executor: executor::Handle,
    pub(crate) sequencer_cometbft_client: SequencerClient,
    pub(crate) sequencer_namespace: Namespace,
    pub(crate) shutdown: CancellationToken,
}

impl Builder {
    /// Creates a new [`Reader`] instance,
    pub(crate) fn build(self) -> eyre::Result<Reader> {
        let Self {
            celestia_block_time,
            celestia_http_endpoint,
            celestia_token,
            executor,
            sequencer_cometbft_client,
            sequencer_namespace,
            shutdown,
        } = self;

        let block_verifier = BlockVerifier::new(sequencer_cometbft_client);

        let celestia_client = create_celestia_client(celestia_http_endpoint, &celestia_token)
            .wrap_err("failed initializing client for Celestia HTTP RPC")?;

        let latest_celestia_heights =
            stream_latest_heights(celestia_client.clone(), celestia_block_time);

        Ok(Reader {
            block_verifier,
            celestia_client,
            latest_celestia_heights,
            executor,
            sequencer_namespace,
            shutdown,
        })
    }
}

fn create_celestia_client(endpoint: String, bearer_token: &str) -> eyre::Result<CelestiaClient> {
    use celestia_client::jsonrpsee::http_client::{
        HeaderMap,
        HttpClientBuilder,
    };
    let mut headers = HeaderMap::new();
    let auth_value = format!("Bearer {bearer_token}").parse().wrap_err(
        "failed to construct Authorization header value from provided Celestia bearer token",
    )?;
    headers.insert(http::header::AUTHORIZATION, auth_value);
    let client = HttpClientBuilder::default()
        .set_headers(headers)
        .build(endpoint)
        .wrap_err("failed constructing Celestia JSONRPC HTTP Client")?;
    Ok(client)
}
