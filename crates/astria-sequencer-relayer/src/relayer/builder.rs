use std::{
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use astria_core::generated::sequencerblock::v1alpha1::sequencer_service_client::SequencerServiceClient;
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::jsonrpsee::http_client::HttpClient as CelestiaClient;
use sequencer_client::HttpClient as SequencerClient;
use tonic::transport::{
    Endpoint,
    Uri,
};

use super::state::State;
use crate::validator::Validator;

pub(crate) struct Builder {
    pub(crate) shutdown_token: tokio_util::sync::CancellationToken,
    pub(crate) celestia_endpoint: String,
    pub(crate) celestia_bearer_token: String,
    pub(crate) cometbft_endpoint: String,
    pub(crate) sequencer_poll_period: Duration,
    pub(crate) sequencer_grpc_endpoint: String,
    pub(crate) validator_key_path: Option<String>,
    pub(crate) pre_submit_path: PathBuf,
    pub(crate) post_submit_path: PathBuf,
}

impl Builder {
    /// Instantiates a `Relayer`.
    pub(crate) fn build(self) -> eyre::Result<super::Relayer> {
        let Self {
            shutdown_token,
            celestia_endpoint,
            celestia_bearer_token,
            cometbft_endpoint,
            sequencer_grpc_endpoint,
            validator_key_path,
            sequencer_poll_period,
            pre_submit_path,
            post_submit_path,
        } = self;
        let sequencer_cometbft_client = SequencerClient::new(&*cometbft_endpoint)
            .wrap_err("failed constructing cometbft http client")?;

        let sequencer_grpc_client = {
            let uri: Uri = sequencer_grpc_endpoint
                .parse()
                .wrap_err("failed parsing provided sequencer grpc endpoint as Uri")?;
            let endpoint = Endpoint::from(uri);
            SequencerServiceClient::new(endpoint.connect_lazy())
        };

        let validator = validator_key_path
            .map(Validator::from_path)
            .transpose()
            .wrap_err("failed to get validator info from file")?;

        let celestia_client = create_celestia_client(celestia_endpoint, &celestia_bearer_token)
            .wrap_err("failed creating client to interact with Celestia Node JSONRPC")?;

        let state = Arc::new(State::new());

        Ok(super::Relayer {
            shutdown_token,
            sequencer_cometbft_client,
            sequencer_grpc_client,
            sequencer_poll_period,
            celestia_client,
            validator,
            state,
            pre_submit_path,
            post_submit_path,
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
