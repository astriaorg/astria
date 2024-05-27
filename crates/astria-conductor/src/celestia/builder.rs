//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use std::time::Duration;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use jsonrpsee::http_client::HttpClient as CelestiaClient;
use tendermint_rpc::HttpClient as SequencerClient;
use tokio_util::sync::CancellationToken;

use super::Reader;
use crate::executor;

pub(crate) struct Builder {
    pub(crate) celestia_block_time: Duration,
    pub(crate) celestia_http_endpoint: String,
    pub(crate) celestia_token: String,
    pub(crate) executor: executor::Handle,
    pub(crate) sequencer_cometbft_client: SequencerClient,
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
            shutdown,
        } = self;

        let celestia_client = create_celestia_client(celestia_http_endpoint, &celestia_token)
            .wrap_err("failed initializing client for Celestia HTTP RPC")?;

        Ok(Reader {
            celestia_block_time,
            celestia_client,
            executor,
            sequencer_cometbft_client,
            shutdown,
        })
    }
}

fn create_celestia_client(endpoint: String, bearer_token: &str) -> eyre::Result<CelestiaClient> {
    use jsonrpsee::http_client::{
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
