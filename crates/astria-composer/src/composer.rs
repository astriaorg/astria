use std::net::SocketAddr;

use astria_core::sequencer::v1::transaction::action::SequenceAction;
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    sync::mpsc::Sender,
    task::JoinError,
};
use tracing::{
    error,
    info,
};

use crate::{
    api::{
        self,
        ApiServer,
    },
    searcher::Searcher,
    Config,
};

/// Composer is a service responsible for submitting transactions to the Astria
/// Shared Sequencer.
pub struct Composer {
    /// `ApiServer` is used for monitoring status of the Composer service.
    api_server: ApiServer,
    /// Searcher establishes connections to individual rollup nodes, receiving
    /// pending transactions from them and wraps them as sequencer transactions
    /// for submission.
    searcher: Searcher,
    /// The handle to communicate `SequenceActions` to the Executor
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: ExecutorHandle,
}

struct ExecutorHandle {
    send_bundles: Sender<SequenceAction>,
}

impl Composer {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// An error is returned if the searcher fails to be initialized.
    /// See `[Searcher::from_config]` for its error scenarios.
    pub fn from_config(cfg: &Config) -> eyre::Result<Self> {
        let (serialized_rollup_transactions_tx, serialized_rollup_transactions_rx) =
            tokio::sync::mpsc::channel(256);

        let executor_handle = ExecutorHandle {
            send_bundles: serialized_rollup_transactions_tx.clone(),
        };

        let searcher = Searcher::from_config(
            cfg,
            executor_handle.send_bundles.clone(),
            serialized_rollup_transactions_rx,
        )
        .wrap_err("failed to initialize searcher")?;

        let searcher_status = searcher.subscribe_to_state();

        let api_server = api::start(cfg.api_listen_addr, searcher_status);
        info!(
            listen_addr = %api_server.local_addr(),
            "API server listening"
        );

        Ok(Self {
            api_server,
            searcher,
            executor_handle,
        })
    }

    /// Returns the socket address the api server is served over
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Runs the composer.
    ///
    /// Currently only exits if the api server or searcher stop unexpectedly.
    pub async fn run_until_stopped(self) {
        let Self {
            api_server,
            searcher,
            executor_handle,
        } = self;

        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });
        let searcher_task = tokio::spawn(searcher.run());
        let _ = executor_handle.send_bundles;

        tokio::select! {
            o = api_task => report_exit("api server", o),
            o = searcher_task => report_exit("searcher", o),
        }
    }
}

fn report_exit(task_name: &str, outcome: Result<eyre::Result<()>, JoinError>) {
    match outcome {
        Ok(Ok(())) => info!(task = task_name, "task exited successfully"),
        Ok(Err(error)) => {
            error!(%error, task = task_name, "task returned with error");
        }
        Err(error) => {
            error!(%error, task = task_name, "task failed to complete");
        }
    }
}
