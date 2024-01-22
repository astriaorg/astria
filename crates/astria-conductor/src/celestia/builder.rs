//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use celestia_client::celestia_types::nmt::Namespace;
use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use deadpool::managed::Pool;
use http::{
    uri::Scheme,
    Uri,
};
use tokio::sync::{
    mpsc,
    oneshot,
};

use super::{
    Reader,
    ReconstructedBlock,
};
use crate::{
    celestia::block_verifier::BlockVerifier,
    client_provider::ClientProvider,
};

pub(crate) struct ReaderBuilder<
    TCelestiaEndpoint = NoCelestiaEndpoint,
    TCelestiaToken = NoCelestiaToken,
    TExecutorChannel = NoExecutorChannel,
    TRollupNamespace = NoRollupNamespace,
    TSequencerClientPool = NoSequencerClientPool,
    TSequencerNamespace = NoSequencerNamespace,
    TShutdown = NoShutdown,
> {
    celestia_endpoint: TCelestiaEndpoint,
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
        use celestia_client::celestia_rpc::{
            Client,
            HeaderClient as _,
        };
        let Self {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint),
            celestia_token: WithCelestiaToken(celestia_token),
            executor_channel: WithExecutorChannel(executor_channel),
            rollup_namespace: WithRollupNamespace(rollup_namespace),
            sequencer_client_pool: WithSequencerClientPool(sequencer_client_pool),
            sequencer_namespace: WithSequencerNamespace(sequencer_namespace),
            shutdown: WithShutdown(shutdown),
        } = self;

        let block_verifier = BlockVerifier::new(sequencer_client_pool);

        let uri = celestia_endpoint
            .parse::<Uri>()
            .wrap_err("failed to parse the provided celestia endpoint as a URL")?;
        let is_tls = matches!(uri.scheme().map(Scheme::as_str), Some("https" | "wss"));

        let ws_endpoint = {
            let mut parts = uri.clone().into_parts();
            parts
                .scheme
                .replace(is_tls.then(wss_scheme).unwrap_or_else(ws_scheme));
            Uri::from_parts(parts)
        }
        .wrap_err("failed constructing websocket endpoint from provided celestia endpoint URL")?;

        let http_endpoint = {
            let mut parts = uri.clone().into_parts();
            parts
                .scheme
                .replace(if is_tls { Scheme::HTTPS } else { Scheme::HTTP });
            Uri::from_parts(parts)
        }
        .wrap_err("failed constructing http endpoint from provided celestia endpoint URL")?;

        let Client::Ws(celestia_ws_client) =
            Client::new(&ws_endpoint.to_string(), Some(&celestia_token))
                .await
                .wrap_err("failed constructing celestia http client")?
        else {
            bail!("expected a celestia HTTP client but got a websocket client");
        };

        let Client::Http(celestia_http_client) =
            Client::new(&http_endpoint.to_string(), Some(&celestia_token))
                .await
                .wrap_err("failed constructing celestia http client")?
        else {
            bail!("expected a celestia HTTP client but got a websocket client");
        };

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let celestia_start_height = celestia_ws_client
            .header_network_head()
            .await
            .wrap_err("failed to get network head from celestia to extract latest head")?
            .header
            .height;

        Ok(Reader {
            executor_channel,
            celestia_http_client,
            celestia_ws_client,
            celestia_start_height,
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
    TCelestiaToken,
    TExecutorChannel,
    TRollupNamespace,
    TSequencerClientPool,
    TSequencerNamespace,
    TShutdown,
>
    ReaderBuilder<
        TCelestiaEndpoint,
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
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint.to_string()),
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
        WithCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaToken,
        TExecutorChannel,
        WithRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor_channel,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        WithSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        WithSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaToken,
        TExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        WithShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor_channel,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        executor_channel: mpsc::UnboundedSender<ReconstructedBlock>,
    ) -> ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaToken,
        WithExecutorChannel,
        TRollupNamespace,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            rollup_namespace,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
pub(crate) struct NoCelestiaToken;
pub(crate) struct WithCelestiaToken(String);
pub(crate) struct NoExecutorChannel;
pub(crate) struct WithExecutorChannel(mpsc::UnboundedSender<ReconstructedBlock>);
pub(crate) struct NoRollupNamespace;
pub(crate) struct WithRollupNamespace(Namespace);
pub(crate) struct NoSequencerClientPool;
pub(crate) struct WithSequencerClientPool(Pool<ClientProvider>);
pub(crate) struct NoSequencerNamespace;
pub(crate) struct WithSequencerNamespace(Namespace);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

fn ws_scheme() -> Scheme {
    "ws".parse::<_>().unwrap()
}

fn wss_scheme() -> Scheme {
    "wss".parse::<_>().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn ws_scheme_utility_works() {
        assert_eq!(&super::ws_scheme(), "ws");
    }
    #[test]
    fn wss_scheme_utility_works() {
        assert_eq!(&super::ws_scheme(), "wss");
    }
}
