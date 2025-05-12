use std::{
    future::Future,
    time::Duration,
};

use astria_core::{
    generated::{
        mempool::v1::transaction_service_server::TransactionServiceServer,
        price_feed::{
            marketmap::v2::query_server::QueryServer as MarketMapQueryServer,
            oracle::v2::query_server::QueryServer as OracleQueryServer,
        },
        sequencerblock::v1::sequencer_service_server::SequencerServiceServer,
    },
    upgrades::v1::Upgrades,
};
use astria_eyre::eyre::{
    self,
    Report,
};
use ibc_proto::ibc::core::{
    channel::v1::query_server::QueryServer as ChannelQueryServer,
    client::v1::query_server::QueryServer as ClientQueryServer,
    connection::v1::query_server::QueryServer as ConnectionQueryServer,
};
use penumbra_tower_trace::remote_addr;
use thiserror::Error;
use tokio::{
    sync::oneshot,
    task::JoinError,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tonic::transport::server::Router;
use tower::layer::util::{
    Identity,
    Stack,
};
use tower_http::cors::CorsLayer;
use tracing::{
    error,
    error_span,
    info,
    info_span,
    instrument,
    warn,
    Instrument as _,
};

use super::{
    mempool,
    optimistic,
    price_feed,
    sequencer::SequencerServer,
};
use crate::{
    app::event_bus::EventBusSubscription,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
    Metrics,
};

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

pub(crate) struct SequencerGrpcServer {
    background_tasks: BackgroundTasks,
    grpc_addr: std::net::SocketAddr,
    grpc_server: Router<Stack<CorsLayer, Identity>>,
    shutdown_rx: oneshot::Receiver<()>,
}

impl SequencerGrpcServer {
    pub(crate) fn builder() -> SequencerGrpcServerBuilder {
        SequencerGrpcServerBuilder {
            storage: None,
            mempool: None,
            upgrades: None,
            metrics: None,
            grpc_addr: None,
            no_optimistic_blocks: false,
            event_bus_subscription: None,
        }
    }

    pub(crate) async fn serve(self) -> Result<(), tonic::transport::Error> {
        self.grpc_server
            .serve_with_shutdown(
                self.grpc_addr,
                shutdown_trigger(self.background_tasks, self.shutdown_rx),
            )
            .await
    }
}

pub(crate) struct SequencerGrpcServerBuilder {
    storage: Option<cnidarium::Storage>,
    mempool: Option<Mempool>,
    upgrades: Option<Upgrades>,
    metrics: Option<&'static Metrics>,
    grpc_addr: Option<std::net::SocketAddr>,
    no_optimistic_blocks: bool,
    event_bus_subscription: Option<EventBusSubscription>,
}

impl SequencerGrpcServerBuilder {
    pub(crate) fn storage(mut self, storage: cnidarium::Storage) -> Self {
        self.storage = Some(storage);
        self
    }

    pub(crate) fn mempool(mut self, mempool: Mempool) -> Self {
        self.mempool = Some(mempool);
        self
    }

    pub(crate) fn upgrades(mut self, upgrades: Upgrades) -> Self {
        self.upgrades = Some(upgrades);
        self
    }

    pub(crate) fn metrics(mut self, metrics: &'static Metrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub(crate) fn grpc_addr(mut self, grpc_addr: std::net::SocketAddr) -> Self {
        self.grpc_addr = Some(grpc_addr);
        self
    }

    pub(crate) fn no_optimistic_blocks(mut self, no_optimistic_blocks: bool) -> Self {
        self.no_optimistic_blocks = no_optimistic_blocks;
        self
    }

    pub(crate) fn event_bus_subscription(
        mut self,
        event_bus_subscription: EventBusSubscription,
    ) -> Self {
        self.event_bus_subscription = Some(event_bus_subscription);
        self
    }

    #[instrument(skip_all, err)]
    pub(crate) fn build(
        self,
    ) -> Result<(SequencerGrpcServer, oneshot::Sender<()>), SequencerServerBuildError> {
        let storage = self
            .storage
            .ok_or(SequencerServerBuildError::MissingStorage)?;
        let mempool = self
            .mempool
            .ok_or(SequencerServerBuildError::MissingMempool)?;
        let upgrades = self
            .upgrades
            .ok_or(SequencerServerBuildError::MissingUpgrades)?;
        let metrics = self
            .metrics
            .ok_or(SequencerServerBuildError::MissingMetrics)?;
        let grpc_addr = self
            .grpc_addr
            .ok_or(SequencerServerBuildError::MissingGrpcAddr)?;
        let event_bus_subscription = self
            .event_bus_subscription
            .ok_or(SequencerServerBuildError::MissingEventBusSubscription)?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let ibc = penumbra_ibc::component::rpc::IbcQuery::<AstriaHost>::new(storage.clone());
        let sequencer_api = SequencerServer::new(storage.clone(), mempool.clone(), upgrades);
        let mempool_api = mempool::Server::new(storage.clone(), mempool, metrics);
        let market_map_api = price_feed::SequencerServer::new(storage.clone());
        let oracle_api = price_feed::SequencerServer::new(storage.clone());
        let cors_layer: CorsLayer = CorsLayer::permissive();

        let mut background_tasks = BackgroundTasks::new();
        let optimistic_block_service = if self.no_optimistic_blocks {
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
            .add_optional_service(optimistic_block_service)
            .add_service(MarketMapQueryServer::new(market_map_api))
            .add_service(OracleQueryServer::new(oracle_api))
            .add_service(TransactionServiceServer::new(mempool_api));

        Ok((
            SequencerGrpcServer {
                background_tasks,
                grpc_addr,
                grpc_server,
                shutdown_rx,
            },
            shutdown_tx,
        ))
    }
}

#[expect(
    clippy::enum_variant_names,
    reason = "error variant names are clearer and more accurate with repeated prefix"
)]
#[derive(Debug, Error)]
pub(crate) enum SequencerServerBuildError {
    #[error("sequencer server builder missing storage")]
    MissingStorage,
    #[error("sequencer server builder missing mempool")]
    MissingMempool,
    #[error("sequencer server builder missing upgrades")]
    MissingUpgrades,
    #[error("sequencer server builder missing metrics")]
    MissingMetrics,
    #[error("sequencer server builder missing gRPC address")]
    MissingGrpcAddr,
    #[error("sequencer server builder missing event bus subscription")]
    MissingEventBusSubscription,
}

async fn shutdown_trigger(
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
                let panic_msg = res.err().map(Report::new).map(tracing::field::display);
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

#[instrument(skip_all)]
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
        warn!(
            tasks = background_tasks.display_running_tasks(),
            "background tasks did not finish during shutdown window and will be aborted",
        );
        background_tasks.abort_all();
    };

    info!("reached shutdown target");
}

#[cfg(test)]
mod tests {
    use telemetry::Metrics as _;

    use super::*;
    use crate::app::benchmark_and_test_utils::AppInitializer;

    async fn dummy_server_builder() -> SequencerGrpcServerBuilder {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let (app, storage) = AppInitializer::new().init().await;
        SequencerGrpcServer::builder()
            .storage(storage)
            .mempool(app.mempool())
            .upgrades(Upgrades::default())
            .metrics(metrics)
            .grpc_addr("0.0.0.0:0".parse().unwrap())
            .no_optimistic_blocks(false)
            .event_bus_subscription(app.subscribe_to_events())
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_storage() {
        let Err(err) = SequencerGrpcServerBuilder {
            storage: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(err, SequencerServerBuildError::MissingStorage));
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_mempool() {
        let Err(err) = SequencerGrpcServerBuilder {
            mempool: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(err, SequencerServerBuildError::MissingMempool));
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_upgrades() {
        let Err(err) = SequencerGrpcServerBuilder {
            upgrades: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(err, SequencerServerBuildError::MissingUpgrades));
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_metrics() {
        let Err(err) = SequencerGrpcServerBuilder {
            metrics: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(err, SequencerServerBuildError::MissingMetrics));
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_grpc_addr() {
        let Err(err) = SequencerGrpcServerBuilder {
            grpc_addr: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(err, SequencerServerBuildError::MissingGrpcAddr));
    }

    #[tokio::test]
    async fn sequencer_grpc_server_build_fails_if_missing_event_bus_subscription() {
        let Err(err) = SequencerGrpcServerBuilder {
            event_bus_subscription: None,
            ..dummy_server_builder().await
        }
        .build() else {
            panic!("expected error, but got Ok");
        };
        assert!(matches!(
            err,
            SequencerServerBuildError::MissingEventBusSubscription
        ));
    }
}
