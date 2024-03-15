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
    Json,
    Router,
};
use http::status::StatusCode;
use hyper::server::conn::AddrIncoming;
use serde::Serialize;
use tokio::sync::watch;

use crate::relayer;

pub(crate) type ApiServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;

type RelayerState = watch::Receiver<relayer::StateSnapshot>;

#[derive(Clone)]
/// `AppState` is used for as an axum extractor in its method handlers.
struct AppState {
    relayer_state: RelayerState,
}

impl FromRef<AppState> for RelayerState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.relayer_state.clone()
    }
}

pub(crate) fn start(
    socket_addr: SocketAddr,
    relayer_state: RelayerState,
    // shutdown: oneshot::Receiver<()>,
) -> ApiServer {
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(AppState {
            relayer_state,
        });
    axum::Server::bind(&socket_addr).serve(app.into_make_service())
}

#[allow(clippy::unused_async)] // Permit because axum handlers must be async
async fn get_healthz(State(relayer_state): State<RelayerState>) -> Healthz {
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
#[allow(clippy::unused_async)] // Permit because axum handlers must be async
async fn get_readyz(State(relayer_state): State<RelayerState>) -> Readyz {
    let is_relayer_online = relayer_state.borrow().is_ready();
    if is_relayer_online {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}

#[allow(clippy::unused_async)] // Permit because axum handlers must be async
async fn get_status(State(relayer_state): State<RelayerState>) -> Json<relayer::StateSnapshot> {
    Json(relayer_state.borrow().clone())
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
