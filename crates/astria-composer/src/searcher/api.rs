use std::net::SocketAddr;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use thiserror::Error;

use super::State as SearcherState;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("hyper error")]
    ServerError(#[from] hyper::Error),
}

pub(super) async fn run(api_url: &SocketAddr, state: &SearcherState) -> Result<(), ApiError> {
    let api_router = Router::new()
        .route("/healthz", get(healthz))
        .with_state(state.clone()); // TODO: should use an Arc here instead of cloning, otherwise state is stale

    axum::Server::bind(api_url)
        .serve(api_router.into_make_service())
        .await
        .map_err(ApiError::ServerError)
}

pub(super) enum Healthz {
    Ok,
    Degraded,
}

impl IntoResponse for Healthz {
    fn into_response(self) -> axum::response::Response {
        #[derive(Debug, Serialize)]
        struct HealthzBody {
            status: &'static str,
        }
        let (status, msg) = match self {
            Self::Ok => (axum::http::StatusCode::OK, "ok"),
            Self::Degraded => (axum::http::StatusCode::GATEWAY_TIMEOUT, "degraded"),
        };
        let mut response = axum::Json(HealthzBody {
            status: msg,
        })
        .into_response();
        *response.status_mut() = status;
        response
    }
}

pub(super) async fn healthz(_state: State<SearcherState>) -> Healthz {
    // TODO: check against state
    Healthz::Ok
}
