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

use crate::bridge;

pub(crate) type ApiServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;

type BridgeState = watch::Receiver<bridge::StateSnapshot>;

#[derive(Clone)]
/// `AppState` is used for as an axum extractor in its method handlers.
struct AppState {
    bridge_state: BridgeState,
}

impl FromRef<AppState> for BridgeState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.bridge_state.clone()
    }
}

pub(crate) fn start(socket_addr: SocketAddr, bridge_state: BridgeState) -> ApiServer {
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(AppState {
            bridge_state,
        });
    axum::Server::bind(&socket_addr).serve(app.into_make_service())
}

#[allow(clippy::unused_async)] // Permit because axum handlers must be async
async fn get_healthz(State(bridge_state): State<BridgeState>) -> Healthz {
    if bridge_state.borrow().is_healthy() {
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
async fn get_readyz(State(bridge_state): State<BridgeState>) -> Readyz {
    let is_bridge_online = bridge_state.borrow().is_ready();
    if is_bridge_online {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}

#[allow(clippy::unused_async)] // Permit because axum handlers must be async
async fn get_status(State(bridge_state): State<BridgeState>) -> Json<bridge::StateSnapshot> {
    Json(bridge_state.borrow().clone())
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
