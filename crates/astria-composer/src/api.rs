use std::net::SocketAddr;

use axum::{
    extract::{
        FromRef,
        State,
    },
    response::{
        IntoResponse,
        Response,
    },
    routing::{
        get,
        IntoMakeService,
    },
    Router,
};
use hyper::server::conn::AddrIncoming;
use serde::Serialize;
use tokio::sync::watch;
use tracing::debug;

use crate::composer_status::ComposerStatus;

pub(super) type ApiServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;

type ComposerStatusWatch = watch::Receiver<ComposerStatus>;

/// `AppState` is an axum extractor
#[derive(Clone)]
struct AppState {
    composer_status: ComposerStatusWatch,
}

impl FromRef<AppState> for ComposerStatusWatch {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.composer_status.clone()
    }
}

pub(super) fn start(listen_addr: SocketAddr, composer_status: ComposerStatusWatch) -> ApiServer {
    let app = Router::new()
        .route("/readyz", get(readyz))
        .with_state(AppState {
            composer_status,
        });
    axum::Server::bind(&listen_addr).serve(app.into_make_service())
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

// axum does not allow non-async handlers. This attribute can be removed
// once this method contains `await` statements.
#[allow(clippy::unused_async)]
async fn readyz(State(composer_status): State<ComposerStatusWatch>) -> Readyz {
    debug!("received readyz request");
    if composer_status.borrow().is_ready() {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}
