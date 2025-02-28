use std::{
    future::{
        Future,
        IntoFuture as _,
    },
    net::SocketAddr,
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use axum::{
    extract::{
        FromRef,
        State,
    },
    response::{
        IntoResponse,
        Response,
    },
    routing::get,
    Json,
    Router,
};
use futures::FutureExt as _;
use http::status::StatusCode;
use serde::Serialize;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::relayer;

pub(super) async fn serve(
    socket_addr: &str,
    relayer_state: watch::Receiver<relayer::StateSnapshot>,
    shutdown_token: CancellationToken,
) -> eyre::Result<Serve> {
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(AppState {
            relayer_state,
        });
    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .wrap_err_with(|| format!("failed to bind TCP socket at `{socket_addr}`"))?;
    let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown_token.cancelled_owned());
    let local_addr = serve
        .local_addr()
        .wrap_err("bound TCP listener failed to yield local address")?;
    Ok(Serve {
        local_addr,
        fut: serve.into_future().boxed(),
    })
}

/// A wrapper around a type-erased [`axum::Serve::serve`] future.
pub(super) struct Serve {
    local_addr: SocketAddr,
    fut: futures::future::BoxFuture<'static, std::io::Result<()>>,
}

impl Serve {
    pub(super) fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Future for Serve {
    type Output = std::io::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.fut.as_mut().poll(cx)
    }
}

#[derive(Clone)]
/// `AppState` is used for as an axum extractor in its method handlers.
struct AppState {
    relayer_state: watch::Receiver<relayer::StateSnapshot>,
}

impl FromRef<AppState> for watch::Receiver<relayer::StateSnapshot> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.relayer_state.clone()
    }
}

#[instrument(skip_all)]
async fn get_healthz(
    State(relayer_state): State<watch::Receiver<relayer::StateSnapshot>>,
) -> Healthz {
    if relayer_state.borrow().is_healthy() {
        Healthz::Ok
    } else {
        Healthz::Degraded
    }
}

/// Handler of a call to `/readyz`.
///
/// Returns `Readyz::Ok` if all of the following conditions are met:
///
/// + there is a current sequencer height (implying a block from sequencer was received)
/// + there is a current data availability height (implying a height was received from the DA)
#[instrument(skip_all)]
async fn get_readyz(
    State(relayer_state): State<watch::Receiver<relayer::StateSnapshot>>,
) -> Readyz {
    let is_relayer_online = relayer_state.borrow().is_ready();
    if is_relayer_online {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}

#[instrument(skip_all)]
async fn get_status(
    State(relayer_state): State<watch::Receiver<relayer::StateSnapshot>>,
) -> Json<relayer::StateSnapshot> {
    Json(*relayer_state.borrow())
}

enum Healthz {
    Ok,
    Degraded,
}

impl IntoResponse for Healthz {
    fn into_response(self) -> Response {
        #[derive(Debug, Serialize)]
        struct ReadyzBody {
            status: &'static str,
        }
        let (status, msg) = match self {
            Self::Ok => (StatusCode::OK, "ok"),
            Self::Degraded => (StatusCode::INTERNAL_SERVER_ERROR, "degraded"),
        };
        let mut response = Json(ReadyzBody {
            status: msg,
        })
        .into_response();
        *response.status_mut() = status;
        response
    }
}

enum Readyz {
    Ok,
    NotReady,
}

impl IntoResponse for Readyz {
    fn into_response(self) -> Response {
        #[derive(Debug, Serialize)]
        struct ReadyzBody {
            status: &'static str,
        }
        let (status, msg) = match self {
            Self::Ok => (StatusCode::OK, "ok"),
            Self::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "not ready"),
        };
        let mut response = Json(ReadyzBody {
            status: msg,
        })
        .into_response();
        *response.status_mut() = status;
        response
    }
}
