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

type RelayerState = watch::Receiver<relayer::State>;

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

pub(crate) fn start(port: u16, relayer_state: RelayerState) -> ApiServer {
    let socket_addr = SocketAddr::from(([127, 0, 0, 1], port));
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(AppState {
            relayer_state,
        });
    axum::Server::bind(&socket_addr).serve(app.into_make_service())
}

async fn get_healthz(State(relayer_status): State<RelayerState>) -> Healthz {
    match relayer_status.borrow().is_ready() {
        true => Healthz::Ok,
        false => Healthz::Degraded,
    }
}

/// Handler of a call to `/readyz`.
///
/// Returns `Readyz::Ok` if all of the following conditions are met:
///
/// + there is a current sequencer height (implying a block from sequencer was received)
/// + there is a current data availability height (implying a height was received from the DA)
async fn get_readyz(State(relayer_status): State<RelayerState>) -> Readyz {
    let is_relayer_online = relayer_status.borrow().is_ready();
    if is_relayer_online {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}

async fn get_status(State(relayer_status): State<RelayerState>) -> Json<serde_json::Value> {
    let relayer::State {
        data_availability_connected,
        sequencer_connected,
        current_sequencer_height,
        current_data_availability_height,
    } = *relayer_status.borrow();
    Json(serde_json::json!({
        "data_availability_connected": data_availability_connected,
        "sequencer_connected": sequencer_connected,
        "current_sequencer_height": current_sequencer_height,
        "current_data_availability_height": current_data_availability_height,
    }))
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
            Self::Degraded => (StatusCode::GATEWAY_TIMEOUT, "degraded"),
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
