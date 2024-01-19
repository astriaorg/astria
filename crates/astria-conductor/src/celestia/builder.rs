//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use celestia_client::celestia_types::nmt::Namespace;
use deadpool::managed::Pool;
use eyre::{
    self,
    WrapErr as _,
};
use http::{
    uri::Scheme,
    Uri,
};
use tokio::sync::oneshot;

use super::Reader;
use crate::{
    celestia::block_verifier::BlockVerifier,
    client_provider::ClientProvider,
    executor,
};

pub(crate) struct ReaderBuilder<
    TCelestiaEndpoint = NoCelestiaEndpoint,
    TCelestiaToken = NoCelestiaToken,
    TExecutor = NoExecutor,
    TSequencerClientPool = NoSequencerClientPool,
    TSequencerNamespace = NoSequencerNamespace,
    TShutdown = NoShutdown,
> {
    celestia_endpoint: TCelestiaEndpoint,
    celestia_token: TCelestiaToken,
    executor: TExecutor,
    sequencer_client_pool: TSequencerClientPool,
    sequencer_namespace: TSequencerNamespace,
    shutdown: TShutdown,
}

impl
    ReaderBuilder<
        WithCelestiaEndpoint,
        WithCelestiaToken,
        WithExecutor,
        WithSequencerClientPool,
        WithSequencerNamespace,
        WithShutdown,
    >
{
    /// Creates a new Reader instance and returns a command sender.
    pub(crate) fn build(self) -> eyre::Result<Reader> {
        let Self {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint),
            celestia_token: WithCelestiaToken(celestia_token),
            executor: WithExecutor(executor),
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

        Ok(Reader {
            executor,
            celestia_http_endpoint: http_endpoint.to_string(),
            celestia_ws_endpoint: ws_endpoint.to_string(),
            celestia_auth_token: celestia_token,
            block_verifier,
            sequencer_namespace,
            shutdown,
        })
    }
}

impl ReaderBuilder {
    pub(super) fn new() -> Self {
        ReaderBuilder {
            celestia_endpoint: NoCelestiaEndpoint,
            celestia_token: NoCelestiaToken,
            executor: NoExecutor,
            sequencer_client_pool: NoSequencerClientPool,
            sequencer_namespace: NoSequencerNamespace,
            shutdown: NoShutdown,
        }
    }
}

impl<
    TCelestiaEndpoint,
    TCelestiaToken,
    TExecutor,
    TSequencerClientPool,
    TSequencerNamespace,
    TShutdown,
>
    ReaderBuilder<
        TCelestiaEndpoint,
        TCelestiaToken,
        TExecutor,
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
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint: WithCelestiaEndpoint(celestia_endpoint.to_string()),
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
        TCelestiaEndpoint,
        WithCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaEndpoint,
        TCelestiaToken,
        TExecutor,
        WithSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        WithSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaEndpoint,
        TCelestiaToken,
        TExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        WithShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            executor,
            sequencer_client_pool,
            sequencer_namespace,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
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
        TCelestiaEndpoint,
        TCelestiaToken,
        WithExecutor,
        TSequencerClientPool,
        TSequencerNamespace,
        TShutdown,
    > {
        let Self {
            celestia_endpoint,
            celestia_token,
            sequencer_client_pool,
            sequencer_namespace,
            shutdown,
            ..
        } = self;
        ReaderBuilder {
            celestia_endpoint,
            celestia_token,
            executor: WithExecutor(executor),
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
pub(crate) struct NoExecutor;
pub(crate) struct WithExecutor(executor::Handle);
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
        assert_eq!(&super::wss_scheme(), "wss");
    }
}
