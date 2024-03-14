use std::{io, net::SocketAddr};
use std::time::Duration;

use astria_core::sequencer::v1::transaction::action::SequenceAction;
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{
        error::SendTimeoutError,
        Sender,
    },
    task::JoinError,
};
use tonic::{
    Request,
    Response,
};

use tracing::{
    error,
    info,
};
use astria_core::generated::composer::v1alpha1::composer_service_server::{ComposerService, ComposerServiceServer};
use astria_core::generated::composer::v1alpha1::SubmitSequenceActionsRequest;
use astria_core::sequencer::v1::asset::default_native_asset_id;
use astria_core::sequencer::v1::RollupId;

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
    /// `Searcher` establishes connections to individual rollup nodes, receiving
    /// pending transactions from them and wraps them as sequencer transactions
    /// for submission.
    searcher: Searcher,
    /// The handle to communicate `SequenceActions` to the Executor
    /// This is at the Composer level to allow its sharing to various different collectors.
    executor_handle: ExecutorHandle,
    /// `GrpcCollectorListener` is the tcp connection on which the gRPC collector is running
    grpc_collector_listener: TcpListener,
}

#[derive(Clone)]
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
    pub async fn from_config(cfg: &Config) -> eyre::Result<Self> {
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

        let grpc_collector_listener = TcpListener::bind(cfg.grpc_collector_addr).await?;

        let api_server = api::start(cfg.api_listen_addr, searcher_status);
        info!(
            listen_addr = %api_server.local_addr(),
            "API server listening"
        );

        Ok(Self {
            api_server,
            searcher,
            executor_handle,
            grpc_collector_listener,
        })
    }

    /// Returns the socket address the api server is served over
    pub fn local_addr(&self) -> SocketAddr {
        self.api_server.local_addr()
    }

    /// Returns the socker address the grpc collector is served over
    /// # Error: Returns an error if the listener is not bound
    pub fn grpc_collector_local_addr(&self) -> io::Result<SocketAddr> {
        self.grpc_collector_listener.local_addr()
    }

    /// Runs the composer.
    ///
    /// Currently only exits if the api server or searcher stop unexpectedly.
    pub async fn run_until_stopped(self) {
        let Self {
            api_server,
            searcher,
            executor_handle,
            grpc_collector_listener,
        } = self;

        let api_task =
            tokio::spawn(async move { api_server.await.wrap_err("api server ended unexpectedly") });
        let searcher_task = tokio::spawn(searcher.run());
        let _ = executor_handle.send_bundles;

        // run the grpc server
        let composer_service = ComposerServiceServer::new(executor_handle);
        let grpc_server = tonic::transport::Server::builder().add_service(composer_service);
        let grpc_server_handler = tokio::spawn(async move {
            grpc_server
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(
                    grpc_collector_listener,
                ))
                .await
        });

        tokio::select! {
            o = api_task => report_exit("api server", o),
            o = searcher_task => report_exit("searcher", o),
            o = grpc_server_handler => println!("grpc server ended: {:?}", o),
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

#[async_trait::async_trait]
impl ComposerService for ExecutorHandle {
    async fn submit_sequence_actions(
        &self,
        request: Request<SubmitSequenceActionsRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let submit_sequence_actions_request = request.into_inner();
        if submit_sequence_actions_request.sequence_actions.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "No sequence actions provided",
            ));
        }

        // package the sequence actions into a SequenceAction and send it to the searcher
        for sequence_action in submit_sequence_actions_request.sequence_actions {
            let sequence_action = SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(sequence_action.rollup_id),
                data: sequence_action.tx_bytes,
                fee_asset_id: default_native_asset_id(),
            };

            match self
                .send_bundles
                .send_timeout(sequence_action, Duration::from_millis(500))
                .await
            {
                Ok(()) => {}
                Err(SendTimeoutError::Timeout(_seq_action)) => {
                    return Err(tonic::Status::deadline_exceeded(
                        "timeout while sending txs to searcher",
                    ));
                }
                Err(SendTimeoutError::Closed(_seq_action)) => {
                    return Err(tonic::Status::unavailable("searcher is not available"));
                }
            }
        }

        Ok(Response::new(()))
    }
}
