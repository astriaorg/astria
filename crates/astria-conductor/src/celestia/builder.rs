//! Boilerplate to construct a [`super::Reader`] via a type-state builder.

use std::str::FromStr;

use celestia_client::celestia_types::nmt::Namespace;
use deadpool::managed::Pool;
use eyre::{
    self,
    WrapErr as _,
};
use http::{
    uri::{
        PathAndQuery,
        Scheme,
    },
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

        let Endpoints {
            http: http_endpoint,
            websocket: ws_endpoint,
        } = celestia_endpoint.parse::<Endpoints>().wrap_err(
            "failed constructing Celestia HTTP and websocket RPC endpoints from provided URI",
        )?;

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

#[derive(Debug, PartialEq, Eq)]
struct Endpoints {
    http: Uri,
    websocket: Uri,
}

impl FromStr for Endpoints {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = s.parse::<Uri>().wrap_err("failed to parse as URI")?;
        let is_tls = matches!(uri.scheme().map(Scheme::as_str), Some("https" | "wss"));

        let http = make_uri(uri.clone(), EndpointKind::Http, is_tls)
            .wrap_err("failed constructing http endpoint from parsed URI")?;
        let websocket = make_uri(uri.clone(), EndpointKind::Ws, is_tls)
            .wrap_err("failed constructing websocket endpoint parsed URI")?;

        Ok(Self {
            http,
            websocket,
        })
    }
}

enum EndpointKind {
    Http,
    Ws,
}

fn make_uri(uri: Uri, kind: EndpointKind, tls: bool) -> Result<Uri, http::uri::InvalidUriParts> {
    let mut parts = uri.clone().into_parts();
    let scheme = match (kind, tls) {
        (EndpointKind::Http, true) => Scheme::HTTPS,
        (EndpointKind::Http, false) => Scheme::HTTP,
        (EndpointKind::Ws, true) => wss_scheme(),
        (EndpointKind::Ws, false) => ws_scheme(),
    };
    parts.scheme.replace(scheme);
    // `Uri::from_parts` fails if scheme is set but path_and_query is empty.
    parts
        .path_and_query
        .get_or_insert(PathAndQuery::from_static("/"));
    Uri::from_parts(parts)
}

#[cfg(test)]
mod tests {
    use http::{
        uri::{
            Authority,
            Parts,
            PathAndQuery,
            Scheme,
        },
        Uri,
    };

    use super::{
        ws_scheme,
        wss_scheme,
        Endpoints,
    };

    #[test]
    fn ws_scheme_utility_works() {
        assert_eq!(&ws_scheme(), "ws");
    }

    #[test]
    fn wss_scheme_utility_works() {
        assert_eq!(&wss_scheme(), "wss");
    }

    #[test]
    fn strings_are_parsed_as_endpoints() {
        // This runs `astria.org` and `192.168.0.1` as inputs to various tests
        // that should all lead to the same pair of endpoints.
        //
        // The following inputs are constructed:
        //
        // astria.org
        // astria.org/
        // http://astria.org
        // https://astria.org
        // ws://astria.org
        // wss://astria.org
        //
        // 192.168.0.1
        // 192.168.0.1/
        // http://192.168.0.1
        // https://192.168.0.1
        // ws://192.168.0.1
        // wss://192.168.0.1
        assert_authority_is_parsed_as_endpoints("astria.org");
        assert_authority_is_parsed_as_endpoints("192.168.0.1");
    }

    #[track_caller]
    fn assert_endpoints_parsed(s: &str, expected: &Endpoints, assert_msg: &'static str) {
        let actual = s
            .parse::<Endpoints>()
            .expect("passed string should be a valid URI");
        assert_eq!(&actual, expected, "{assert_msg}");
    }

    fn make_endpoints(authority: &'static str, tls: bool) -> Endpoints {
        let http_endpoint = {
            let mut parts = Parts::default();
            parts
                .scheme
                .replace(if tls { Scheme::HTTPS } else { Scheme::HTTP });
            parts.authority.replace(Authority::from_static(authority));
            parts.path_and_query.replace(PathAndQuery::from_static("/"));
            Uri::from_parts(parts).expect("all required URI parts should be set")
        };
        let ws_endpoint = {
            let mut parts = Parts::default();
            parts
                .scheme
                .replace(if tls { wss_scheme() } else { ws_scheme() });
            parts.authority.replace(Authority::from_static(authority));
            parts.path_and_query.replace(PathAndQuery::from_static("/"));
            Uri::from_parts(parts).expect("all required URI parts should be set")
        };
        Endpoints {
            http: http_endpoint,
            websocket: ws_endpoint,
        }
    }

    #[track_caller]
    fn assert_authority_is_parsed_as_endpoints(authority: &'static str) {
        let non_tls_endpoints = make_endpoints(authority, false);
        let tls_endpoints = make_endpoints(authority, true);
        assert_endpoints_parsed(
            authority,
            &non_tls_endpoints,
            "URI with only authority but no scheme nor path should give expected http and ws \
             endpoints",
        );
        assert_endpoints_parsed(
            &format!("{authority}/"),
            &non_tls_endpoints,
            "URI with authority and empty path but no scheme should lead to expected ws and http \
             endpoints",
        );
        assert_endpoints_parsed(
            &format!("ws://{authority}"),
            &non_tls_endpoints,
            "URI with authority and ws scheme but no path should lead to expected ws and http \
             endpoints",
        );
        assert_endpoints_parsed(
            &format!("http://{authority}"),
            &non_tls_endpoints,
            "URI with authority and http scheme but no path should lead to expected ws and http \
             endpoints",
        );
        assert_endpoints_parsed(
            &format!("wss://{authority}"),
            &tls_endpoints,
            "URI with authority and wss scheme but no path should lead to expected wss and https \
             endpoints",
        );
        assert_endpoints_parsed(
            &format!("https://{authority}"),
            &tls_endpoints,
            "URI with authority and https scheme but no path should lead to expected ws and http \
             endpoints",
        );
    }
}
