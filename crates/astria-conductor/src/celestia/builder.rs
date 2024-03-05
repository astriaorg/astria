//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use celestia_client::celestia_types::nmt::Namespace;
use deadpool::managed::Pool;
use tokio::sync::oneshot;

use super::Reader;
use crate::{
    celestia::block_verifier::BlockVerifier,
    client_provider::ClientProvider,
    executor,
};

pub(crate) struct ReaderBuilder<
    TCelestiaHttpEndpoint = NoCelestiaHttpEndpoint,
    TCelestiaWebsocketEndpoint = NoCelestiaWebsocketEndpoint,
    TCelestiaToken = NoCelestiaToken,
    TExecutor = NoExecutor,
    TSequencerClientPool = NoSequencerClientPool,
    TSequencerNamespace = NoSequencerNamespace,
    TShutdown = NoShutdown,
> {
    celestia_http_endpoint: TCelestiaHttpEndpoint,
    celestia_websocket_endpoint: TCelestiaWebsocketEndpoint,
    celestia_token: TCelestiaToken,
    executor: TExecutor,
    sequencer_client_pool: TSequencerClientPool,
    sequencer_namespace: TSequencerNamespace,
    shutdown: TShutdown,
}

impl
    ReaderBuilder<
        WithCelestiaHttpEndpoint,
        WithCelestiaWebsocketEndpoint,
        WithCelestiaToken,
        WithExecutor,
        WithSequencerClientPool,
        WithSequencerNamespace,
        WithShutdown,
    >
{
    /// Creates a new Reader instance and returns a command sender.
    pub(crate) fn build(self) -> Reader {
        let Self {
            celestia_http_endpoint: WithCelestiaHttpEndpoint(celestia_http_endpoint),
            celestia_websocket_endpoint: WithCelestiaWebsocketEndpoint(celestia_ws_endpoint),
            celestia_token: WithCelestiaToken(celestia_token),
            executor: WithExecutor(executor),
            sequencer_client_pool: WithSequencerClientPool(sequencer_client_pool),
            sequencer_namespace: WithSequencerNamespace(sequencer_namespace),
            shutdown: WithShutdown(shutdown),
        } = self;

        let block_verifier = BlockVerifier::new(sequencer_client_pool);

        Reader {
            executor,
            celestia_http_endpoint,
            celestia_ws_endpoint,
            celestia_auth_token: celestia_token,
            block_verifier,
            sequencer_namespace,
            shutdown,
        }
    }
}

impl ReaderBuilder {
    pub(super) fn new() -> Self {
        ReaderBuilder {
            celestia_http_endpoint: NoCelestiaHttpEndpoint,
            celestia_websocket_endpoint: NoCelestiaWebsocketEndpoint,
            celestia_token: NoCelestiaToken,
            executor: NoExecutor,
            sequencer_client_pool: NoSequencerClientPool,
            sequencer_namespace: NoSequencerNamespace,
            shutdown: NoShutdown,
        }
    }
}

impl<
    TCelestiaHttpEndpoint,
    TCelestiaWebsocketEndpoint,
    TCelestiaToken,
    TExecutor,
    TSequencerClientPool,
    TSequencerNamespace,
    TShutdown,
>
    ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    >
{
    pub(crate) fn celestia_http_endpoint(
        self,
        celestia_http_endpoint: &str,
    ) -> ReaderBuilder<
        WithCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint: WithCelestiaHttpEndpoint(celestia_http_endpoint.to_string()),
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn celestia_websocket_endpoint(
        self,
        celestia_websocket_endpoint: &str,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        WithCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint: WithCelestiaWebsocketEndpoint(
                celestia_websocket_endpoint.to_string(),
            ),
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn celestia_token(
        self,
        celestia_token: &str,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        WithCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token: WithCelestiaToken(celestia_token.to_string()),
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn sequencer_client_pool(
        self,
        sequencer_client_pool: Pool<ClientProvider>,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        WithSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool: WithSequencerClientPool(sequencer_client_pool),
            sequencer_namespace,
            shutdown,
        }
    }

    pub(crate) fn sequencer_namespace(
        self,
        sequencer_namespace: Namespace,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        WithSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace: WithSequencerNamespace(sequencer_namespace),
            shutdown,
        }
    }

    pub(crate) fn shutdown(
        self,
        shutdown: oneshot::Receiver<()>,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        WithShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown: WithShutdown(shutdown),
        }
    }

    pub(crate) fn executor(
        self,
        executor: executor::Handle,
    ) -> ReaderBuilder<
        TCelestiaHttpEndpoint,
        TCelestiaWebsocketEndpoint,
        TCelestiaToken,
        WithExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_http_endpoint,
            celestia_websocket_endpoint,
            celestia_token,
            executor: WithExecutor(executor),
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
        }
    }
}

pub(crate) struct NoCelestiaHttpEndpoint;
pub(crate) struct WithCelestiaHttpEndpoint(String);
pub(crate) struct NoCelestiaWebsocketEndpoint;
pub(crate) struct WithCelestiaWebsocketEndpoint(String);
pub(crate) struct NoCelestiaToken;
pub(crate) struct WithCelestiaToken(String);
pub(crate) struct NoExecutor;
pub(crate) struct WithExecutor(executor::Handle);
pub(crate) struct NoSequencerClientPool;
pub(crate) struct WithSequencerClientPool(Pool<ClientProvider>);
pub(crate) struct NoSequencerNamespace;
pub(crate) struct WithSequencerNamespace(Namespace);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);
