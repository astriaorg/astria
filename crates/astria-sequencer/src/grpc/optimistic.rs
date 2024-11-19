use std::{
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use astria_core::{
    generated::sequencerblock::optimistic::v1alpha1::{
        optimistic_block_service_server::OptimisticBlockService,
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::{
        optimistic::SequencerBlockCommit,
        SequencerBlock,
    },
    Protobuf,
};
use astria_eyre::{
    eyre,
    eyre::eyre,
};
use tokio::{
    sync::{
        mpsc,
        mpsc::{
            error::SendError,
            Receiver,
        },
    },
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tonic::{
    codegen::tokio_stream::{
        wrappers::ReceiverStream,
        Stream,
    },
    Request,
    Response,
    Status,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
};

use crate::app::event_bus::{
    EventBus,
    EventReceiver,
};

const STREAM_TASKS_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

struct StartOptimisticBlockStreamRequest {
    rollup_id: RollupId,
    tx: mpsc::Sender<Result<GetOptimisticBlockStreamResponse, Status>>,
}

struct StartBlockCommitmentStreamRequest {
    tx: mpsc::Sender<tonic::Result<GetBlockCommitmentStreamResponse>>,
}

enum NewStreamRequest {
    OptimisticBlockStream(StartOptimisticBlockStreamRequest),
    BlockCommitmentStream(StartBlockCommitmentStreamRequest),
}

pub(crate) struct OptimisticBlockInner {
    event_bus: EventBus,
    stream_request_receiver: mpsc::Receiver<NewStreamRequest>,
    stream_tasks: JoinSet<Result<(), eyre::Report>>,
    cancellation_token: CancellationToken,
}

// the below streams are free standing functions as implementing them as methods on
// OptimisticBlockInner will cause lifetime issues with the self reference. This is because the
// Joinset requires that the future being spawned should have a static lifetime.
async fn block_commitment_stream(
    mut finalize_block_sender: EventReceiver<Arc<SequencerBlockCommit>>,
    tx: mpsc::Sender<tonic::Result<GetBlockCommitmentStreamResponse>>,
    cancellation_token: CancellationToken,
) -> Result<(), eyre::Report> {
    loop {
        tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                break Ok(());
            }
            finalize_block_res = finalize_block_sender.receive() => {
                match finalize_block_res {
                    Ok(finalize_block) => {
                        let sequencer_block_commit = finalize_block
                            .clone();

                        let get_block_commitment_stream_response = GetBlockCommitmentStreamResponse {
                            commitment: Some(sequencer_block_commit.to_raw()),
                        };

                        if let Err(e) = tx.send(Ok(get_block_commitment_stream_response)).await {
                            break Err(eyre!(
                                "grpc stream receiver for block commitment has been dropped with error: {e}"
                            ));
                        };
                    },
                    Err(e) => {
                        break Err(eyre!("finalize block sender has been dropped with error: {e}"));
                    }
                }
            },
        }
    }
}

async fn optimistic_stream(
    mut process_proposal_block_receiver: EventReceiver<Arc<SequencerBlock>>,
    rollup_id: RollupId,
    tx: mpsc::Sender<Result<GetOptimisticBlockStreamResponse, Status>>,
    cancellation_token: CancellationToken,
) -> Result<(), eyre::Report> {
    loop {
        tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                break Ok(());
            }
            process_proposal_block_res = process_proposal_block_receiver.receive() => {
                match process_proposal_block_res {
                    Ok(process_proposal_block) => {
                        let filtered_optimistic_block = process_proposal_block
                            .clone()
                            .to_filtered_block(vec![rollup_id]);
                        let raw_filtered_optimistic_block = filtered_optimistic_block.into_raw();

                        let get_optimistic_block_stream_response = GetOptimisticBlockStreamResponse {
                            block: Some(raw_filtered_optimistic_block),
                        };

                        if let Err(e) = tx.send(Ok(get_optimistic_block_stream_response)).await {
                            break Err(eyre!(
                                "grpc stream receiver for optimistic block has been dropped with error: {e}"
                            ));
                        };
                    },
                    Err(e) => {
                        break Err(eyre!("process proposal block sender has been dropped with error: {e}"));
                    }
                }
            },
        }
    }
}

impl OptimisticBlockInner {
    fn new(
        event_bus: EventBus,
        stream_request_receiver: mpsc::Receiver<NewStreamRequest>,
        server_cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            event_bus,
            stream_request_receiver,
            stream_tasks: JoinSet::new(),
            cancellation_token: server_cancellation_token,
        }
    }

    fn handle_optimistic_block_stream_request(
        &mut self,
        request: StartOptimisticBlockStreamRequest,
    ) {
        let StartOptimisticBlockStreamRequest {
            rollup_id,
            tx,
        } = request;

        let process_proposal_block_receiver = self.event_bus.subscribe_process_proposal_blocks();

        self.stream_tasks.spawn(optimistic_stream(
            process_proposal_block_receiver,
            rollup_id,
            tx,
            self.cancellation_token.child_token(),
        ));
    }

    fn handle_block_commitment_stream_request(
        &mut self,
        request: StartBlockCommitmentStreamRequest,
    ) {
        let StartBlockCommitmentStreamRequest {
            tx,
        } = request;

        let finalize_block_sender = self.event_bus.subscribe_finalize_blocks();

        self.stream_tasks.spawn(block_commitment_stream(
            finalize_block_sender,
            tx,
            self.cancellation_token.child_token(),
        ));
    }

    pub(crate) async fn run(&mut self) -> eyre::Result<()> {
        loop {
            tokio::select! {
                biased;
                () = self.cancellation_token.cancelled() => {
                    break;
                },
                Some(inner_stream_request) = self.stream_request_receiver.recv() => {
                    match inner_stream_request {
                        NewStreamRequest::OptimisticBlockStream(request) => {
                            self.handle_optimistic_block_stream_request(request);
                        }
                        NewStreamRequest::BlockCommitmentStream(request) => {
                            self.handle_block_commitment_stream_request(request);
                        }
                    }
                },
            }
        }

        self.shutdown().await
    }

    #[instrument(skip_all)]
    async fn shutdown(&mut self) -> Result<(), eyre::Report> {
        match tokio::time::timeout(STREAM_TASKS_SHUTDOWN_DURATION, async {
            while let Some(joined_tasks) = self.stream_tasks.join_next().await {
                match joined_tasks {
                    Ok(Ok(())) => {
                        debug!("stream task has been joined successfully");
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "stream task has been joined with an error");
                    }
                    Err(e) => {
                        error!(error = %e, "stream task has been joined with an error");
                    }
                }
            }
        })
        .await
        {
            Ok(()) => {
                info!("all stream tasks have been joined successfully");
            }
            Err(e) => {
                error!(error = %e, "stream tasks failed to shut down in time");
                self.stream_tasks.abort_all();
            }
        }

        Ok(())
    }
}

pub(crate) struct OptimisticBlockFacade {
    stream_request_sender: mpsc::Sender<NewStreamRequest>,
}

impl OptimisticBlockFacade {
    fn new(stream_request_sender: mpsc::Sender<NewStreamRequest>) -> Self {
        Self {
            stream_request_sender,
        }
    }

    pub(crate) fn new_optimistic_block_service(
        event_bus: EventBus,
        cancellation_token: CancellationToken,
    ) -> (Self, OptimisticBlockInner) {
        let (tx, rx) = mpsc::channel(128);

        let facade = Self::new(tx);
        let inner = OptimisticBlockInner::new(event_bus, rx, cancellation_token);

        (facade, inner)
    }

    #[instrument(skip_all)]
    async fn spawn_optimistic_block_stream(
        &self,
        get_optimistic_block_stream_request: GetOptimisticBlockStreamRequest,
    ) -> eyre::Result<Receiver<tonic::Result<GetOptimisticBlockStreamResponse>>> {
        let rollup_id = {
            let rollup_id = get_optimistic_block_stream_request
                .rollup_id
                .ok_or_else(|| Status::invalid_argument("rollup id is required"))?;

            RollupId::try_from_raw(rollup_id)
                .map_err(|e| eyre!(format!("invalid rollup id: {e}")))?
        };

        let (tx, rx) =
            tokio::sync::mpsc::channel::<tonic::Result<GetOptimisticBlockStreamResponse>>(128);

        let request = NewStreamRequest::OptimisticBlockStream(StartOptimisticBlockStreamRequest {
            rollup_id,
            tx,
        });

        self.stream_request_sender.send(request).await?;

        Ok(rx)
    }

    #[instrument(skip_all)]
    async fn spawn_block_commitment_stream_request(
        &self,
    ) -> Result<
        Receiver<tonic::Result<GetBlockCommitmentStreamResponse>>,
        SendError<NewStreamRequest>,
    > {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<tonic::Result<GetBlockCommitmentStreamResponse>>(128);

        let request = NewStreamRequest::BlockCommitmentStream(StartBlockCommitmentStreamRequest {
            tx,
        });

        self.stream_request_sender.send(request).await?;

        Ok(rx)
    }
}

#[async_trait::async_trait]
impl OptimisticBlockService for OptimisticBlockFacade {
    type GetBlockCommitmentStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetBlockCommitmentStreamResponse, Status>> + Send>>;
    type GetOptimisticBlockStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetOptimisticBlockStreamResponse, Status>> + Send>>;

    #[instrument(skip_all)]
    async fn get_optimistic_block_stream(
        self: Arc<Self>,
        request: Request<GetOptimisticBlockStreamRequest>,
    ) -> Result<Response<Self::GetOptimisticBlockStreamStream>, Status> {
        let get_optimistic_block_stream_request = request.into_inner();

        let rx = match self
            .spawn_optimistic_block_stream(get_optimistic_block_stream_request)
            .await
        {
            Ok(rx) => rx,
            Err(e) => {
                return Err(Status::internal(format!(
                    "failed to create optimistic block stream: {e}"
                )));
            }
        };

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetOptimisticBlockStreamStream
        ))
    }

    #[instrument(skip_all)]
    async fn get_block_commitment_stream(
        self: Arc<Self>,
        _request: Request<GetBlockCommitmentStreamRequest>,
    ) -> Result<Response<Self::GetBlockCommitmentStreamStream>, Status> {
        let rx = match self.spawn_block_commitment_stream_request().await {
            Ok(rx) => rx,
            Err(e) => {
                return Err(Status::internal(format!(
                    "failed to create block commitment stream: {e}"
                )));
            }
        };

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetBlockCommitmentStreamStream
        ))
    }
}
