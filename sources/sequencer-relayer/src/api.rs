use std::net::SocketAddr;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use http::status::StatusCode;
use serde::Serialize;
use tokio::sync::watch::Receiver;

use crate::relayer::State as RelayerState;

pub async fn start(addr: SocketAddr, relayer_state: Receiver<RelayerState>) {
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(relayer_state);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("app server exited unexpectedly");
}

async fn get_healthz(State(relayer_status): State<Receiver<RelayerState>>) -> Healthz {
    match relayer_status.borrow().is_ready() {
        true => Healthz::Ok,
        false => Healthz::Degraded,
    }
}

async fn get_readyz(State(relayer_status): State<Receiver<RelayerState>>) -> Readyz {
    match relayer_status.borrow().is_ready() {
        true => Readyz::Ok,
        false => Readyz::NotReady,
    }
}

async fn get_status(State(relayer_status): State<Receiver<RelayerState>>) -> Json<Status> {
    let status = Status::from_relayer_status(&relayer_status.borrow());
    Json(status)
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
        let mut response = Json(ReadyzBody { status: msg }).into_response();
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
        let mut response = Json(ReadyzBody { status: msg }).into_response();
        *response.status_mut() = status;
        response
    }
}

#[derive(Debug, Serialize)]
struct Status {
    current_sequencer_height: Option<u64>,
    current_data_availability_height: Option<u64>,
}

impl Status {
    fn from_relayer_status(relayer_status: &RelayerState) -> Self {
        Self {
            current_sequencer_height: relayer_status.current_sequencer_height,
            current_data_availability_height: relayer_status.current_data_availability_height,
        }
    }
}
