use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        eyre,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use penumbra_tower_trace::{
    trace::request_span,
    v038::RequestExt as _,
};
use telemetry::metrics::register_histogram_global;
use tendermint::v0_38::abci::ConsensusRequest;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::{
        oneshot,
        watch,
    },
    task::JoinHandle,
};
use tower_abci::v038::Server;
use tracing::{
    debug,
    error,
    error_span,
    info,
    info_span,
    instrument,
};

use crate::{
    app::App,
    config::Config,
    mempool::Mempool,
    metrics::Metrics,
    service,
};

pub struct Sequencer;

struct RunningGrpcServer {
    pub handle: JoinHandle<Result<(), tonic::transport::Error>>,
    pub shutdown_tx: oneshot::Sender<()>,
}

struct RunningAbciServer {
    pub handle: JoinHandle<()>,
    pub shutdown_rx: oneshot::Receiver<()>,
}

impl Sequencer {
    /// Builds and runs the sequencer until it is either stopped by a signal or an error occurs.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// - Database file does not exist, or cannot be loaded into storage
    /// - The app fails to initialize
    /// - Info service fails to initialize
    /// - The server builder fails to return a server
    /// - The gRPC address cannot be parsed
    /// - The gRPC server fails to exit properly
    pub async fn spawn(config: Config, metrics: &'static Metrics) -> Result<()> {
        let mut signals = spawn_signal_handler();
        let initialize_fut = Self::initialize(config, metrics);
        select! {
            _ = signals.stop_rx.changed() => {
                info_span!("initialize").in_scope(|| info!("shutting down sequencer"));
                Ok(())
            }

            result = initialize_fut => {
                let (grpc_server, abci_server) = result?;
                Self::run_until_stopped(abci_server, grpc_server, &mut signals).await
            }
        }
    }

    async fn run_until_stopped(
        abci_server: RunningAbciServer,
        grpc_server: RunningGrpcServer,
        signals: &mut SignalReceiver,
    ) -> Result<()> {
        select! {
            _ = signals.stop_rx.changed() => {
                info_span!("run_until_stopped").in_scope(|| info!("shutting down sequencer"));
            }

            _ = abci_server.shutdown_rx => {
                info_span!("run_until_stopped").in_scope(|| error!("ABCI server task exited, this shouldn't happen"));
            }
        }

        grpc_server
            .shutdown_tx
            .send(())
            .map_err(|()| eyre!("failed to send shutdown signal to grpc server"))?;
        grpc_server
            .handle
            .await
            .wrap_err("grpc server task failed")?
            .wrap_err("grpc server failed")?;
        abci_server.handle.abort();
        Ok(())
    }

    #[instrument(skip_all)]
    async fn initialize(
        config: Config,
        metrics: &'static Metrics,
    ) -> Result<(RunningGrpcServer, RunningAbciServer)> {
        cnidarium::register_metrics();
        register_histogram_global("cnidarium_get_raw_duration_seconds");
        register_histogram_global("cnidarium_nonverifiable_get_raw_duration_seconds");

        let substore_prefixes = vec![penumbra_ibc::IBC_SUBSTORE_PREFIX];

        let storage = cnidarium::Storage::load(
            config.db_filepath.clone(),
            substore_prefixes
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        )
        .await
        .map_err(anyhow_to_eyre)
        .wrap_err("failed to load storage backing chain state")?;
        let snapshot = storage.latest_snapshot();

        let mempool = Mempool::new(metrics, config.mempool_parked_max_tx_count);

        let app = App::new(snapshot, mempool.clone(), metrics)
            .await
            .wrap_err("failed to initialize app")?;

        let event_bus_subscription = app.subscribe_to_events();

        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { service::Consensus::new(storage, app, queue).run().await }
            }));
        let mempool_service = service::Mempool::new(storage.clone(), mempool.clone(), metrics);
        let info_service =
            service::Info::new(storage.clone()).wrap_err("failed initializing info service")?;
        let snapshot_service = service::Snapshot;

        let abci_server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_eyre("server builder didn't return server; are all fields set?")?;

        let (grpc_shutdown_tx, grpc_shutdown_rx) = tokio::sync::oneshot::channel();
        let (abci_shutdown_tx, abci_shutdown_rx) = tokio::sync::oneshot::channel();

        let grpc_addr = config
            .grpc_addr
            .parse()
            .wrap_err("failed to parse grpc_addr address")?;

        // TODO(janis): need a mechanism to check and report if the grpc server setup failed.
        // right now it's fire and forget and the grpc server is only reaped if sequencer
        // itself is taken down.
        let grpc_server_handle = tokio::spawn(crate::grpc::serve(
            storage.clone(),
            mempool,
            grpc_addr,
            config.no_optimistic_blocks,
            event_bus_subscription,
            grpc_shutdown_rx,
        ));

        debug!(config.listen_addr, "starting sequencer");

        let listen_addr = config.listen_addr.clone();
        let abci_server_handle = tokio::spawn(async move {
            match abci_server.listen_tcp(listen_addr).await {
                Ok(()) => {
                    // this shouldn't happen, as there isn't a way for the ABCI server to exit
                    info_span!("abci_server").in_scope(|| info!("ABCI server exited successfully"));
                }
                Err(e) => {
                    error_span!("abci_server")
                        .in_scope(|| error!(err = e.as_ref(), "ABCI server exited with error"));
                }
            }
            let _ = abci_shutdown_tx.send(());
        });

        let grpc_server = RunningGrpcServer {
            handle: grpc_server_handle,
            shutdown_tx: grpc_shutdown_tx,
        };
        let abci_server = RunningAbciServer {
            handle: abci_server_handle,
            shutdown_rx: abci_shutdown_rx,
        };

        Ok((grpc_server, abci_server))
    }
}

struct SignalReceiver {
    stop_rx: watch::Receiver<()>,
}

fn spawn_signal_handler() -> SignalReceiver {
    let (stop_tx, stop_rx) = watch::channel(());
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).expect(
            "setting a SIGINT listener should always work on unix; is this running on unix?",
        );
        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
        );
        loop {
            select! {
                _ = sigint.recv() => {
                    info!("received SIGINT");
                    let _ = stop_tx.send(());
                }
                _ = sigterm.recv() => {
                    info!("received SIGTERM");
                    let _ = stop_tx.send(());
                }
            }
        }
    });

    SignalReceiver {
        stop_rx,
    }
}
