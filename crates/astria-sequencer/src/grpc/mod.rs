pub(crate) mod optimistic;
pub(crate) mod sequencer;
mod state_ext;
pub(crate) mod storage;

use std::time::Duration;

use astria_core::generated::astria::sequencerblock::v1::sequencer_service_server::SequencerServiceServer;
use astria_eyre::eyre::WrapErr as _;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    info_span,
    warn,
};

use crate::{
    app::event_bus::EventBusSubscription,
    grpc::sequencer::SequencerServer,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
};

// we provide a shutdown time mainly for the optimistic block service tasks to shutdown
// gracefully
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(1500);
const SHUTDOWN_SPAN: &str = "grpc_server_shutdown";

pub(crate) fn start_server(
    storage: &cnidarium::Storage,
    mempool: Mempool,
    grpc_addr: std::net::SocketAddr,
    no_optimistic_blocks: bool,
    event_bus_subscription: EventBusSubscription,
    shutdown_rx: oneshot::Receiver<()>,
) -> JoinHandle<astria_eyre::Result<(), tonic::transport::Error>> {
    use ibc_proto::ibc::core::{
        channel::v1::query_server::QueryServer as ChannelQueryServer,
        client::v1::query_server::QueryServer as ClientQueryServer,
        connection::v1::query_server::QueryServer as ConnectionQueryServer,
    };
    use penumbra_tower_trace::remote_addr;
    use tower_http::cors::CorsLayer;

    let ibc = penumbra_ibc::component::rpc::IbcQuery::<AstriaHost>::new(storage.clone());
    let sequencer_api = SequencerServer::new(storage.clone(), mempool);
    let cors_layer: CorsLayer = CorsLayer::permissive();

    let optimistic_streams_cancellation_token = CancellationToken::new();

    let (optimistic_block_service, mut optimistic_block_task) = if no_optimistic_blocks {
        (None, None)
    } else {
        let (optimistic_block_service, optimistic_block_task) = optimistic::new_service(
            event_bus_subscription,
            optimistic_streams_cancellation_token.child_token(),
        );

        (Some(optimistic_block_service), Some(optimistic_block_task))
    };

    // TODO: setup HTTPS?
    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                let addr = remote_addr.to_string();
                tracing::error_span!("grpc", addr)
            } else {
                tracing::error_span!("grpc")
            }
        })
        // (from Penumbra) Allow HTTP/1, which will be used by grpc-web connections.
        // This is particularly important when running locally, as gRPC
        // typically uses HTTP/2, which requires HTTPS. Accepting HTTP/2
        // allows local applications such as web browsers to talk to pd.
        .accept_http1(true)
        // (from Penumbra) Add permissive CORS headers, so pd's gRPC services are accessible
        // from arbitrary web contexts, including from localhost.
        .layer(cors_layer)
        .add_service(ClientQueryServer::new(ibc.clone()))
        .add_service(ChannelQueryServer::new(ibc.clone()))
        .add_service(ConnectionQueryServer::new(ibc.clone()))
        .add_service(SequencerServiceServer::new(sequencer_api))
        .add_optional_service(optimistic_block_service);

    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");

    tokio::task::spawn(grpc_server.serve_with_shutdown(grpc_addr, async move {
        let reason = tokio::select! {
            biased;
            _ = shutdown_rx => {
                Ok("grpc server shutting down")
            },
            res = async { optimistic_block_task.as_mut().unwrap().await }, if optimistic_block_task.is_some() => {
                match res {
                    Ok(()) => {
                        Ok("optimistic block inner handle task exited successfully")
                    },
                    Err(e) => {
                        Err(e).wrap_err("optimistic block inner handle task exited with error")
                    }
                }
            }
        };
        optimistic_streams_cancellation_token.cancel();

        if optimistic_block_task.is_some() {
            // give time for the optimistic block service to shutdown all the streaming tasks.
            let handle = optimistic_block_task.as_mut().unwrap();
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut *handle).await {
                Ok(Ok(())) => {
                    info!("optimistic block service task shutdown gracefully");
                },
                Ok(Err(e)) => {
                    warn!(%e, "optimistic block service has panicked")
                }
                Err(e) => {
                    error!(%e, "optimistic block service task didn't shutdown in time");
                    handle.abort();
                }
            }
        }

        info_span!(SHUTDOWN_SPAN).in_scope(|| {
            match reason {
                Ok(reason) => {
                    info!(reason);
                }
                Err(reason) => {
                    warn!("{}", reason.to_string());
                }
            };
        });
    }))
}
