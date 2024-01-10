//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use std::time::Duration;

use celestia_client::celestia_types::nmt::Namespace;
use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use deadpool::managed::Pool;
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_util::task::JoinMap;

use super::{
    Reader,
    SequencerBlockSubset,
};
use crate::{
    client_provider::ClientProvider,
    data_availability::block_verifier::BlockVerifier,
};

pub(crate) struct ReaderBuilder<
    TCelestiaEndpoint = NoCelestiaEndpoint,
    TCelestiaPollInterval = NoCelestiaPollInterval,
    TCelestiaToken = NoCelestiaToken,
    TExecutorChannel = NoExecutorChannel,
    TRollupNamespace = NoRollupNamespace,
    TSequencerClientPool = NoSequencerClientPool,
    TSequencerNamespace = NoSequencerNamespace,
    TShutdown = NoShutdown,
> {
    celestia_endpoint: TCelestiaEndpoint,
    celestia_poll_interval: TCelestiaPollInterval,
    celestia_token: TCelestiaToken,
    executor_channel: TExecutorChannel,
    rollup_namespace: TRollupNamespace,
    sequencer_client_pool: TSequencerClientPool,
    sequencer_namespace: TSequencerNamespace,
    shutdown: TShutdown,
}

impl
    ReaderBuilder<
        WithCelestiaEndpoint,
        WithCelestiaPollInterval,
        WithCelestiaToken,
        WithExecutorChannel,
        WithRollupNamespace,
        WithSequencerClientPool,
        WithSequencerNamespace,
        WithShutdown,
    >
{
    /// Creates a new Reader instance and returns a command sender.
    pub(crate) async fn build(self) -> eyre::Result<Reader> {
        use celestia_client::celestia_rpc::HeaderClient as _;
        let Self {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint),
            celestia_poll_interval: WithCelestiaPollInterval(celestia_poll_interval),
            celestia_token: WithCelestiaToken(celestia_token),
            executor_channel: WithExecutorChannel(executor_channel),
            rollup_namespace: WithRollupNamespace(rollup_namespace),
            sequencer_client_pool: WithSequencerClientPool(sequencer_client_pool),
            sequencer_namespace: WithSequencerNamespace(sequencer_namespace),
            shutdown: WithShutdown(shutdown),
        } = self;

        let block_verifier = BlockVerifier::new(sequencer_client_pool);

        let celestia_client::celestia_rpc::Client::Http(celestia_client) =
            celestia_client::celestia_rpc::Client::new(&celestia_endpoint, Some(&celestia_token))
                .await
                .wrap_err("failed constructing celestia http client")?
        else {
            bail!("expected a celestia HTTP client but got a websocket client");
        };

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let current_block_height = celestia_client
            .header_network_head()
            .await
            .wrap_err("failed to get network head from celestia to extract latest head")?
            .header
            .height;

        Ok(Reader {
            celestia_client,
            celestia_poll_interval,
            current_block_height,
            executor_channel,
            get_latest_height: None,
            fetch_sequencer_blobs_at_height: JoinMap::new(),
            verify_sequencer_blobs_and_assemble_rollups: JoinMap::new(),
            block_verifier,
            sequencer_namespace,
            rollup_namespace,
            shutdown,
        })
    }
}

impl ReaderBuilder {
    pub(super) fn new() -> Self {
        ReaderBuilder {
            celestia_endpoint: NoCelestiaEndpoint,
            celestia_poll_interval: NoCelestiaPollInterval,
            celestia_token: NoCelestiaToken,
            executor_channel: NoExecutorChannel,
            rollup_namespace: NoRollupNamespace,
            sequencer_client_pool: NoSequencerClientPool,
            sequencer_namespace: NoSequencerNamespace,
            shutdown: NoShutdown,
        }
    }
}

impl<
    TCelestiaEndpoint,
    TCelestiaPollInterval,
    TCelestiaToken,
    TExecutorChannel,
    TRollupNamespace,
    TSequencerClientPool,
    TSequencerNamespace,
    TShutdown,
>
    ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    >
{
    pub(crate) fn celestia_endpoint(
        self,
        celestia_endpoint: &str,
    ) -> ReaderBuilder<
        WithCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_token,
            celestia_poll_interval,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint.to_string()),
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn celestia_poll_interval(
        self,
        celestia_poll_interval: Duration,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        WithCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval: WithCelestiaPollInterval(celestia_poll_interval),
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn celestia_token(
        self,
        celestia_token: &str,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        WithCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token: WithCelestiaToken(celestia_token.to_string()),
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn rollup_namespace(
        self,
        rollup_namespace: Namespace,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        WithRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace: WithRollupNamespace(rollup_namespace),
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn sequencer_client_pool(
        self,
        sequencer_client_pool: Pool<ClientProvider>,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        WithSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool: WithSequencerClientPool(sequencer_client_pool),
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn sequencer_namespace(
        self,
        sequencer_namespace: Namespace,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        WithSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace: WithSequencerNamespace(sequencer_namespace),
            shutdown,
        }
    }

    pub(crate) fn shutdown(
        self,
        shutdown: oneshot::Receiver<()>,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        WithShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown: WithShutdown(shutdown),
        }
    }

    pub(crate) fn executor_channel(
        self,
        executor_channel: mpsc::UnboundedSender<Vec<SequencerBlockSubset>>,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaPollInterval,
        TCelestiaToken,
        WithExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_poll_interval,
            celestia_token,
            executor_channel: WithExecutorChannel(executor_channel),
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }
}

pub(crate) struct NoCelestiaEndpoint;
pub(crate) struct WithCelestiaEndpoint(String);
pub(crate) struct NoCelestiaPollInterval;
pub(crate) struct WithCelestiaPollInterval(Duration);
pub(crate) struct NoCelestiaToken;
pub(crate) struct WithCelestiaToken(String);
pub(crate) struct NoExecutorChannel;
pub(crate) struct WithExecutorChannel(mpsc::UnboundedSender<Vec<SequencerBlockSubset>>);
pub(crate) struct NoRollupNamespace;
pub(crate) struct WithRollupNamespace(Namespace);
pub(crate) struct NoSequencerClientPool;
pub(crate) struct WithSequencerClientPool(Pool<ClientProvider>);
pub(crate) struct NoSequencerNamespace;
pub(crate) struct WithSequencerNamespace(Namespace);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);
