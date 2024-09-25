use std::time::Duration;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use itertools::Itertools as _;
use tokio::{
    select,
    time::timeout,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    flatten,
    optimistic_executor,
    Config,
    Metrics,
};

pub(super) struct Auctioneer {
    /// Used to signal the service to shutdown
    shutdown_token: CancellationToken,

    /// The different long-running tasks that make up the Auctioneer
    tasks: JoinMap<&'static str, eyre::Result<()>>,
}

impl Auctioneer {
    const AUCTION_DRIVER: &'static str = "auction_driver";
    const OPTIMISTIC_EXECUTOR: &'static str = "optimistic_executor";
    const _BUNDLE_COLLECTOR: &'static str = "bundle_collector";

    /// Creates an [`Auctioneer`] service from a [`Config`] and [`Metrics`].
    pub(super) fn new(
        cfg: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let Config {
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            latency_margin_ms,
            rollup_grpc_endpoint,
            rollup_id,
            ..
        } = cfg;

        let mut tasks = JoinMap::new();

        let optimistic_executor = optimistic_executor::Builder {
            metrics,
            shutdown_token: shutdown_token.clone(),
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            rollup_id,
            optimistic_execution_grpc_endpoint: rollup_grpc_endpoint.clone(),
            bundle_grpc_endpoint: rollup_grpc_endpoint.clone(),
            latency_margin: Duration::from_millis(latency_margin_ms),
        }
        .build()
        .wrap_err("failed to initialize the optimistic executor")?;
        tasks.spawn(Self::OPTIMISTIC_EXECUTOR, optimistic_executor.run());

        Ok(Self {
            shutdown_token,
            tasks,
        })
    }

    /// Runs the [`Auctioneer`] service until it received an exit signal, or one of the constituent
    /// tasks either ends unexpectedly or returns an error.
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let reason = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                Ok("auctioneer received shutdown signal")
            },

            Some((name, res)) = self.tasks.join_next() => {
                flatten(res)
                    .wrap_err_with(|| format!("task `{name}` failed"))
                    .map(|_| "task `{name}` exited unexpectedly")
            }
        };

        match reason {
            Ok(msg) => info!(%msg, "received shutdown signal"),
            Err(err) => error!(%err, "shutting down due to error"),
        }

        self.shutdown().await;
        Ok(())
    }

    /// Initiates shutdown of the Auctioneer and waits for all the constituent tasks to shut down.
    async fn shutdown(mut self) {
        self.shutdown_token.cancel();

        let shutdown_loop = async {
            while let Some((name, res)) = self.tasks.join_next().await {
                let message = "task shut down";
                match flatten(res) {
                    Ok(()) => {
                        info!(name, message)
                    }
                    Err(err) => {
                        error!(name, %err, message)
                    }
                }
            }
        };

        info!("signalling all tasks to shut down; waiting 25 seconds for exit");
        if timeout(Duration::from_secs(25), shutdown_loop)
            .await
            .is_err()
        {
            let tasks = self.tasks.keys().join(", ");
            warn!(
                tasks = format_args!("[{tasks}]"),
                "aborting all tasks that have not yet shut down"
            )
        } else {
            info!("all tasks have shut down regularly");
        }
        info!("shutting down");
    }
}
