use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
};
use serde::Serialize;

use super::State as SearcherState;

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

pub(super) async fn healthz(_state: State<Arc<SearcherState>>) -> Healthz {
    // TODO: check against state
    Healthz::Ok
}
