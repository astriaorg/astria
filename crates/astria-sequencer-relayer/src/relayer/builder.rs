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
use sequencer_client::HttpClient as SequencerClient;
use tonic::transport::{
    Endpoint,
    Uri,
};

use super::{
    state::State,
    CelestiaClientBuilder,
    CelestiaKeys,
};
use crate::{
    validator::Validator,
    IncludeRollup,
};

pub(crate) struct Builder {
    pub(crate) shutdown_token: tokio_util::sync::CancellationToken,
    pub(crate) celestia_app_grpc_endpoint: String,
    pub(crate) celestia_app_key_file: String,
    pub(crate) cometbft_endpoint: String,
    pub(crate) sequencer_poll_period: Duration,
    pub(crate) sequencer_grpc_endpoint: String,
    pub(crate) validator_key_path: Option<String>,
    pub(crate) rollup_filter: IncludeRollup,
    pub(crate) pre_submit_path: PathBuf,
    pub(crate) post_submit_path: PathBuf,
}

impl Builder {
    /// Instantiates a `Relayer`.
    pub(crate) fn build(self) -> eyre::Result<super::Relayer> {
        let Self {
            shutdown_token,
            celestia_app_grpc_endpoint,
            celestia_app_key_file,
            cometbft_endpoint,
            sequencer_poll_period,
            sequencer_grpc_endpoint,
            validator_key_path,
            rollup_filter,
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

        let state = Arc::new(State::new());

        let celestia_client_builder = {
            let uri: Uri = celestia_app_grpc_endpoint
                .parse()
                .wrap_err("failed parsing provided celestia app grpc endpoint as Uri")?;
            let celestia_keys = CelestiaKeys::from_path(celestia_app_key_file)
                .wrap_err("failed to get celestia keys from file")?;
            CelestiaClientBuilder::new(uri, celestia_keys, state.clone())
                .wrap_err("failed to create celestia client builder")?
        };

        Ok(super::Relayer {
            shutdown_token,
            sequencer_cometbft_client,
            sequencer_grpc_client,
            sequencer_poll_period,
            celestia_client_builder,
            validator,
            rollup_filter,
            state,
            pre_submit_path,
            post_submit_path,
        })
    }
}
