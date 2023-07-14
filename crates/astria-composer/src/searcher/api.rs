use std::{
    net::SocketAddr,
    sync::Arc,
};

use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use tokio::sync::broadcast::Receiver;
use tracing::info;

use super::{
    Action,
    Event,
};

pub(super) async fn run(
    api_url: SocketAddr,
    _event_rx: Receiver<Event>,
    _action_rx: Receiver<Action>,
) -> Result<(), hyper::Error> {
    let api_router = Router::new().route("/healthz", get(healthz));
    // TODO: add routes for events and actions

    info!(?api_url, "starting api server");

    axum::Server::bind(&api_url)
        .serve(api_router.into_make_service())
        .await
}

pub(super) enum Healthz {
    Ok,
    _Degraded,
}

impl IntoResponse for Healthz {
    fn into_response(self) -> axum::response::Response {
        #[derive(Debug, Serialize)]
        struct HealthzBody {
            status: &'static str,
        }
        let (status, msg) = match self {
            Self::Ok => (axum::http::StatusCode::OK, "ok"),
            Self::_Degraded => (axum::http::StatusCode::GATEWAY_TIMEOUT, "degraded"),
        };
        let mut response = axum::Json(HealthzBody {
            status: msg,
        })
        .into_response();
        *response.status_mut() = status;
        response
    }
}

pub(super) async fn healthz() -> Healthz {
    // TODO: check against state
    Healthz::Ok
}
