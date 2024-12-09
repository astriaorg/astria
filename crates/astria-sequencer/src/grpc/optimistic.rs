use std::{
    pin::Pin,
    sync::Arc,
};

use astria_core::{
    generated::sequencerblock::optimisticblock::v1alpha1::{
        optimistic_block_service_server::OptimisticBlockService,
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::{
        optimistic_block::SequencerBlockCommit,
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
        mpsc::error::SendError,
    },
    task::JoinSet,
};
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
};

use crate::app::{
    EventBus,
    EventReceiver,
};

type OptimisticBlockStreamResponse = Result<GetOptimisticBlockStreamResponse, Status>;
type BlockCommitmentStreamResponse = Result<GetBlockCommitmentStreamResponse, Status>;

struct GetOptimisticBlockStreamInnerRequest {
    rollup_id: RollupId,
    tx: mpsc::Sender<OptimisticBlockStreamResponse>,
}

struct GetBlockCommitmentStreamInnerRequest {
    tx: mpsc::Sender<BlockCommitmentStreamResponse>,
}

enum InnerStreamRequest {
    GetOptimisticBlockStream(GetOptimisticBlockStreamInnerRequest),
    GetBlockCommitmentStream(GetBlockCommitmentStreamInnerRequest),
}

pub(crate) struct OptimisticBlockInner {
    event_bus: EventBus,
    stream_request_receiver: mpsc::Receiver<InnerStreamRequest>,
    stream_tasks: JoinSet<Result<(), eyre::Report>>,
}

async fn optimistic_stream_task(
    mut process_proposal_block_receiver: EventReceiver<Option<Arc<SequencerBlock>>>,
    rollup_id: RollupId,
    tx: mpsc::Sender<OptimisticBlockStreamResponse>,
) -> Result<(), eyre::Report> {
    loop {
        match process_proposal_block_receiver.receive().await {
            Ok(process_proposal_block) => {
                let filtered_optimistic_block = process_proposal_block
                    .clone()
                    .expect("unexpected None")
                    .to_filtered_block(vec![rollup_id]);
                let raw_filtered_optimistic_block = filtered_optimistic_block.into_raw();

                let get_optimistic_block_stream_response = GetOptimisticBlockStreamResponse {
                    block: Some(raw_filtered_optimistic_block),
                };

                if let Err(e) = tx.send(Ok(get_optimistic_block_stream_response)).await {
                    error!(error = %e, "grpc stream receiver for optimistic block has been dropped");
                    return Err(eyre!(
                        "grpc stream receiver for optimistic block has been dropped"
                    ));
                };
            }
            Err(e) => {
                error!(error = %e, "process proposal block sender has been dropped");
                return Err(eyre!("process proposal block sender has been dropped"));
            }
        }
    }
}

async fn block_commitment_stream_task(
    mut finalize_block_sender: EventReceiver<Option<Arc<SequencerBlockCommit>>>,
    tx: mpsc::Sender<BlockCommitmentStreamResponse>,
) -> Result<(), eyre::Report> {
    loop {
        match finalize_block_sender.receive().await {
            Ok(finalize_block) => {
                let sequencer_block_commit = finalize_block.clone().expect("unexpected None");

                let get_block_commitment_stream_response = GetBlockCommitmentStreamResponse {
                    commitment: Some(sequencer_block_commit.to_raw()),
                };

                if let Err(e) = tx.send(Ok(get_block_commitment_stream_response)).await {
                    error!(error = %e, "grpc stream receiver for block commitment has been dropped");
                    return Err(eyre!(
                        "grpc stream receiver for block commitment has been dropped"
                    ));
                };
            }
            Err(e) => {
                error!(error = %e, "finalize block sender has been dropped");
                return Err(eyre!("finalize block sender has been dropped"));
            }
        }
    }
}

impl OptimisticBlockInner {
    fn new(
        event_bus: EventBus,
        stream_request_receiver: mpsc::Receiver<InnerStreamRequest>,
    ) -> Self {
        Self {
            event_bus,
            stream_request_receiver,
            stream_tasks: JoinSet::new(),
        }
    }

    fn handle_optimistic_block_stream_request(
        &mut self,
        request: GetOptimisticBlockStreamInnerRequest,
    ) {
        let GetOptimisticBlockStreamInnerRequest {
            rollup_id,
            tx,
        } = request;

        let process_proposal_block_receiver = self.event_bus.subscribe_process_proposal_blocks();

        self.stream_tasks.spawn(async move {
            optimistic_stream_task(process_proposal_block_receiver, rollup_id, tx).await
        });
    }

    fn handle_block_commitment_stream_request(
        &mut self,
        request: GetBlockCommitmentStreamInnerRequest,
    ) {
        let GetBlockCommitmentStreamInnerRequest {
            tx,
        } = request;

        let finalize_block_sender = self.event_bus.subscribe_finalize_blocks();

        self.stream_tasks
            .spawn(async move { block_commitment_stream_task(finalize_block_sender, tx).await });
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(inner_stream_request) = self.stream_request_receiver.recv() => {
                    match inner_stream_request {
                        InnerStreamRequest::GetOptimisticBlockStream(request) => {
                            self.handle_optimistic_block_stream_request(request);
                        }
                        InnerStreamRequest::GetBlockCommitmentStream(request) => {
                            self.handle_block_commitment_stream_request(request);
                        }
                    }
                },
                Some(joined_task) = self.stream_tasks.join_next() => {
                    match joined_task {
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
                },
                else => {}
            }
        }
    }
}

pub(crate) struct OptimisticBlockFacade {
    stream_request_sender: mpsc::Sender<InnerStreamRequest>,
}

impl OptimisticBlockFacade {
    fn new(stream_request_sender: mpsc::Sender<InnerStreamRequest>) -> Self {
        Self {
            stream_request_sender,
        }
    }

    pub(crate) fn new_optimistic_block_service(
        event_bus: EventBus,
    ) -> (Self, OptimisticBlockInner) {
        let (tx, rx) = mpsc::channel(128);

        let facade = Self::new(tx);
        let inner = OptimisticBlockInner::new(event_bus, rx);

        (facade, inner)
    }

    async fn send_optimistic_block_stream_request(
        &self,
        rollup_id: RollupId,
        tx: mpsc::Sender<OptimisticBlockStreamResponse>,
    ) -> Result<(), SendError<InnerStreamRequest>> {
        let request =
            InnerStreamRequest::GetOptimisticBlockStream(GetOptimisticBlockStreamInnerRequest {
                rollup_id,
                tx,
            });

        self.stream_request_sender.send(request).await?;

        Ok(())
    }

    async fn send_block_commitment_stream_request(
        &self,
        tx: mpsc::Sender<BlockCommitmentStreamResponse>,
    ) -> Result<(), SendError<InnerStreamRequest>> {
        let request =
            InnerStreamRequest::GetBlockCommitmentStream(GetBlockCommitmentStreamInnerRequest {
                tx,
            });

        self.stream_request_sender.send(request).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl OptimisticBlockService for OptimisticBlockFacade {
    type GetBlockCommitmentStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetBlockCommitmentStreamResponse, Status>> + Send>>;
    type GetOptimisticBlockStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetOptimisticBlockStreamResponse, Status>> + Send>>;

    async fn get_optimistic_block_stream(
        self: Arc<Self>,
        request: Request<GetOptimisticBlockStreamRequest>,
    ) -> Result<Response<Self::GetOptimisticBlockStreamStream>, Status> {
        let get_optimistic_block_stream_request = request.into_inner();

        // The facade creates the stream channels and sends it to the inner optimistic block service
        let rollup_id = {
            let rollup_id = get_optimistic_block_stream_request
                .rollup_id
                .ok_or_else(|| Status::invalid_argument("rollup id is required"))?;

            RollupId::try_from_raw(rollup_id)
                .map_err(|e| Status::invalid_argument(format!("invalid rollup id: {e}")))?
        };

        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<GetOptimisticBlockStreamResponse, Status>>(128);

        if let Err(e) = self
            .send_optimistic_block_stream_request(rollup_id, tx)
            .await
        {
            return Err(Status::internal(format!(
                "failed to send optimistic block stream request: {e}"
            )));
        }

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetOptimisticBlockStreamStream
        ))
    }

    async fn get_block_commitment_stream(
        self: Arc<Self>,
        _request: Request<GetBlockCommitmentStreamRequest>,
    ) -> Result<Response<Self::GetBlockCommitmentStreamStream>, Status> {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<GetBlockCommitmentStreamResponse, Status>>(128);

        if let Err(e) = self.send_block_commitment_stream_request(tx).await {
            return Err(Status::internal(format!(
                "failed to send block commitment stream request: {e}"
            )));
        }

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetBlockCommitmentStreamStream
        ))
    }
}
