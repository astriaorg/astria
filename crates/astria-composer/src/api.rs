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
    Router,
};
use futures::FutureExt as _;
use serde::Serialize;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    instrument,
};

use crate::composer;

pub(super) async fn serve(
    listen_addr: SocketAddr,
    composer_status: watch::Receiver<composer::Status>,
    shutdown_token: CancellationToken,
) -> eyre::Result<Serve> {
    let app = Router::new()
        .route("/readyz", get(readyz))
        .with_state(AppState {
            composer_status,
        });
    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .wrap_err_with(|| format!("failed to bind `{listen_addr}`"))?;
    let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown_token.cancelled_owned());
    let local_addr = serve
        .local_addr()
        .wrap_err("bound TCP listener failed to report local addr")?;
    Ok(Serve {
        local_addr,
        fut: serve.into_future().boxed(),
    })
}

/// A wrapper of a type-erased [`axum::serve::Serve`] future.
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

/// `AppState` is an axum extractor
#[derive(Clone)]
struct AppState {
    composer_status: watch::Receiver<composer::Status>,
}

impl FromRef<AppState> for watch::Receiver<composer::Status> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.composer_status.clone()
    }
}

enum Readyz {
    Ok,
    NotReady,
}

impl IntoResponse for Readyz {
    fn into_response(self) -> Response {
        #[derive(Debug, Serialize)]
        struct ReadyBody {
            status: &'static str,
        }
        let (status, msg) = match self {
            Self::Ok => (axum::http::StatusCode::OK, "ok"),
            Self::NotReady => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready"),
        };
        let mut response = axum::Json(ReadyBody {
            status: msg,
        })
        .into_response();
        *response.status_mut() = status;
        response
    }
}

#[instrument(skip_all)]
async fn readyz(State(composer_status): State<watch::Receiver<composer::Status>>) -> Readyz {
    debug!("received readyz request");
    if composer_status.borrow().is_ready() {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}
