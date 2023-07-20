use std::net::SocketAddr;

use axum::{
    response::IntoResponse,
    routing::{
        get,
        IntoMakeService,
    },
    Router,
};
use hyper::server::conn::AddrIncoming;
use serde::Serialize;

pub(super) type ApiServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;

pub(super) fn start(port: u16) -> ApiServer {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let api_router = Router::new().route("/healthz", get(healthz));
    // TODO: add routes for events and actions
    axum::Server::bind(&addr).serve(api_router.into_make_service())
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
