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
use eyre::WrapErr as _;
use http::status::StatusCode;
use hyper::server::conn::AddrIncoming;
use serde::Serialize;
use tokio::sync::{
    mpsc::UnboundedSender,
    oneshot,
    watch,
};

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

/// Handler of a call to `/readyz`.
///
/// Returns `Readyz::Ok` if all of the following conditions are met:
///
/// + there is at least one subscriber to which sequencer blocks can be gossiped
/// + there is a current sequencer height (implying a block from sequencer was received)
/// + there is a current data availability height (implying a height was received from the DA)
async fn get_readyz(
    State(relayer_status): State<RelayerState>,
    State(gossipnet_query_tx): State<UnboundedSender<network::InfoQuery>>,
) -> Readyz {
    let are_peers_subscribed = match get_number_of_subscribers(gossipnet_query_tx).await {
        Ok(0) => false,
        Ok(_) => true,
        Err(e) => {
            report_err!(
                e,
                "failed querying gossipnet task for its number of subscribers"
            );
            false
        }
    };
    let is_relayer_online = relayer_status.borrow().is_ready();
    if are_peers_subscribed && is_relayer_online {
        Readyz::Ok
    } else {
        Readyz::NotReady
    }
}

async fn get_status(
    State(relayer_status): State<RelayerState>,
    State(gossipnet_query_tx): State<UnboundedSender<network::InfoQuery>>,
) -> Json<serde_json::Value> {
    let relayer::State {
        data_availability_connected,
        sequencer_connected,
        current_sequencer_height,
        current_data_availability_height,
    } = *relayer_status.borrow();
    let number_of_subscribed_peers = match get_number_of_subscribers(gossipnet_query_tx).await {
        Ok(num) => Some(num),
        Err(e) => {
            report_err!(
                e,
                "failed querying gossipnet task for its number of subscribers"
            );
            None
        }
    };
    Json(serde_json::json!({
        "data_availability_connected": data_availability_connected,
        "sequencer_connected": sequencer_connected,
        "current_sequencer_height": current_sequencer_height,
        "current_data_availability_height": current_data_availability_height,
        "number_of_subscribed_peers": number_of_subscribed_peers,
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

async fn get_number_of_subscribers(tx: UnboundedSender<network::InfoQuery>) -> eyre::Result<u64> {
    use eyre::Report;
    use network::InfoQuery;
    let (info_response_tx, info_response_rx) = oneshot::channel();
    tx.send(InfoQuery::NumberOfPeers(info_response_tx))
        .map_err(|e| {
            Report::msg(e.to_string()).wrap_err("failed sending info query to gossipnet task")
        })?;
    info_response_rx
        .await
        .map(|num| {
            num.try_into()
                .expect("number of subscribed peers should never exceed u64::MAX")
        })
        .wrap_err("one shot channel sent to gossipnet dropped before value was returned")
}
