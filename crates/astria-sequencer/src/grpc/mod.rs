use std::{
    future::Future,
    time::Duration,
};

use astria_core::generated::astria::sequencerblock::v1::sequencer_service_server::SequencerServiceServer;
use astria_eyre::eyre;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};
use tokio::{
    sync::oneshot,
    task::JoinError,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    error_span,
    info,
    info_span,
    Instrument as _,
};

use crate::{
    app::event_bus::EventBusSubscription,
    grpc::sequencer::SequencerServer,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
};

pub(crate) mod optimistic;
pub(crate) mod sequencer;
mod state_ext;
pub(crate) mod storage;

/// Time for the background tasks supporting gRPC services to shutdown gracefully before being
/// aborted.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(1500);
const SHUTDOWN_SPAN: &str = "grpc_server_shutdown";

struct BackgroundTasks {
    tasks: JoinMap<&'static str, ()>,
    cancellation_token: CancellationToken,
}

impl BackgroundTasks {
    fn new() -> Self {
        Self {
            tasks: JoinMap::new(),
            cancellation_token: CancellationToken::new(),
        }
    }

    fn abort_all(&mut self) {
        self.tasks.abort_all();
    }

    fn cancel_all(&self) {
        self.cancellation_token.cancel();
    }

    fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    fn display_running_tasks(&self) -> String {
        use itertools::Itertools as _;
        format!("[{}]", self.tasks.keys().format(","))
    }

    fn spawn<F>(&mut self, key: &'static str, task: F)
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        self.tasks.spawn(key, task);
    }

    async fn join_next(&mut self) -> Option<(&'static str, Result<(), JoinError>)> {
        self.tasks.join_next().await
    }
}

pub(crate) async fn serve(
    storage: cnidarium::Storage,
    mempool: Mempool,
    grpc_addr: std::net::SocketAddr,
    no_optimistic_blocks: bool,
    event_bus_subscription: EventBusSubscription,
    shutdown_rx: oneshot::Receiver<()>,
) -> eyre::Result<(), tonic::transport::Error> {
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

    let mut background_tasks = BackgroundTasks::new();
    let optimistic_block_service = if no_optimistic_blocks {
        None
    } else {
        let (service, task) = optimistic::new(
            event_bus_subscription,
            background_tasks.cancellation_token(),
        );
        background_tasks.spawn("OPTIMISTIC", task.run());
        Some(service)
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

    grpc_server
        .serve_with_shutdown(grpc_addr, trigger_shutdown(background_tasks, shutdown_rx))
        .await
}

async fn trigger_shutdown(
    mut background_tasks: BackgroundTasks,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let shutdown_span;
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown_rx => {
                shutdown_span = info_span!(SHUTDOWN_SPAN);
                shutdown_span.in_scope(|| {
                    info!("grpc server received shutdown signal and will shutdown all of its background tasks");
                });
                break;
            }

            Some((task, res)) = background_tasks.join_next() => {
                let panic_msg = res.err().map(eyre::Report::new).map(tracing::field::display);
                error_span!("grpc_background_task_failed").in_scope(|| {
                    error!(
                        panic_msg,
                        task,
                        "background task supporting a grpc service ended unexpectedly; Sequencer will \
                        keep responding to gRPC requests, but there is currently no way to recover \
                        functionality of this service until Sequencer is restarted"
                    );
                });
            }
        }
    }
    perform_shutdown(background_tasks)
        .instrument(shutdown_span)
        .await;
}

async fn perform_shutdown(mut background_tasks: BackgroundTasks) {
    background_tasks.cancel_all();

    if let Ok(()) = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
        while let Some((task, res)) = background_tasks.join_next().await {
            let error = res
                .err()
                .map(eyre::Report::new)
                .map(tracing::field::display);
            info!(
                error,
                task, "background task exited while awaiting shutdown"
            );
        }
    })
    .await
    {
        info!("all background tasks exited during shutdown window");
    } else {
        error!(
            tasks = background_tasks.display_running_tasks(),
            "background tasks did not finish during shutdown window and will be aborted",
        );
        background_tasks.abort_all();
    };

    info!("reached shutdown target");
}
