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
use serde::{
    Deserialize,
    Serialize,
};
use tokio::sync::{
    mpsc::UnboundedSender,
    oneshot,
    watch,
};
use tracing::warn;

use crate::{
    macros::report_err,
    network,
    relayer,
};

pub(crate) type ApiServer = axum::Server<AddrIncoming, IntoMakeService<Router>>;

type RelayerState = watch::Receiver<relayer::State>;
type GossipnetState = UnboundedSender<network::InfoQuery>;

#[derive(Clone)]
/// `AppState` is used for as an axum extractor in its method handlers.
struct AppState {
    relayer_state: RelayerState,
    gossipnet_state: GossipnetState,
}

impl FromRef<AppState> for RelayerState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.relayer_state.clone()
    }
}

impl FromRef<AppState> for GossipnetState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.gossipnet_state.clone()
    }
}

pub(crate) fn start(
    port: u16,
    relayer_state: RelayerState,
    gossipnet_state: GossipnetState,
) -> ApiServer {
    let socket_addr = SocketAddr::from(([127, 0, 0, 1], port));
    let app = Router::new()
        .route("/healthz", get(get_healthz))
        .route("/readyz", get(get_readyz))
        .route("/status", get(get_status))
        .with_state(AppState {
            relayer_state,
            gossipnet_state,
        });
    axum::Server::bind(&socket_addr).serve(app.into_make_service())
}

async fn get_healthz(State(relayer_status): State<RelayerState>) -> Healthz {
    match relayer_status.borrow().is_ready() {
        true => Healthz::Ok,
        false => Healthz::Degraded,
    }
}

async fn get_readyz(State(relayer_status): State<RelayerState>) -> Readyz {
    match relayer_status.borrow().is_ready() {
        true => Readyz::Ok,
        false => Readyz::NotReady,
    }
}

async fn get_status(
    State(relayer_status): State<RelayerState>,
    State(info_query_tx): State<UnboundedSender<crate::network::InfoQuery>>,
) -> Json<Status> {
    use crate::network::InfoQuery;
    let relayer::State {
        current_sequencer_height,
        current_data_availability_height,
    } = *relayer_status.borrow();
    let (info_response_tx, info_response_rx) = oneshot::channel();
    if let Err(e) = info_query_tx.send(InfoQuery::NumberOfPeers(info_response_tx)) {
        warn!(error.msg = %e, "failed sending info query to gossip task");
    }
    let number_of_subscribed_peers = match info_response_rx.await {
        Ok(num) => Some(
            num.try_into()
                .expect("number of subscribed peers should never exceed u64::MAX"),
        ),
        Err(e) => {
            report_err!(
                e,
                "oneshot channel to receive number of subscribers from gossip task dropped before \
                 a value was sent"
            );
            None
        }
    };
    Json(Status {
        current_sequencer_height,
        current_data_availability_height,
        number_of_subscribed_peers,
    })
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    pub current_sequencer_height: Option<u64>,
    pub current_data_availability_height: Option<u64>,
    pub number_of_subscribed_peers: Option<u64>,
}
