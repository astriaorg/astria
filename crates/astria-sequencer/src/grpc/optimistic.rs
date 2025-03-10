use std::{
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use astria_core::{
    generated::astria::sequencerblock::optimistic::v1alpha1::{
        optimistic_block_service_server::{
            OptimisticBlockService,
            OptimisticBlockServiceServer,
        },
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
    sequencerblock::{
        optimistic::v1alpha1::SequencerBlockCommit,
        v1::{
            block,
            SequencerBlock,
        },
    },
    Protobuf as _,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tendermint::{
    abci::request::FinalizeBlock,
    Hash,
};
use tokio::{
    sync::mpsc,
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
    error,
    info,
    info_span,
    instrument,
    trace,
    warn,
};

use crate::app::event_bus::{
    EventBusSubscription,
    EventReceiver,
};

const STREAM_TASKS_SHUTDOWN_DURATION: Duration = Duration::from_secs(1);
const OPTIMISTIC_STREAM_SPAN: &str = "optimistic_stream";
const BLOCK_COMMITMENT_STREAM_SPAN: &str = "block_commitment_stream";

type GrpcStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

/// Create a new optimistic block service.
///
/// The service is split into a frontend and backend part,
/// where [`Facade`] wrapped in a [`OptimisticBlockServer<Facade>`] is
/// to be passed to a [`tonic::tranport::Server`], while [`Runner`] is
/// should be spawned as a separate task.
///
/// The [`Runner`] keeps track of all stream that are requested on
/// the gRPC server and are forwarded to it via the [`Facade`].
pub(super) fn new(
    event_bus_subscription: EventBusSubscription,
    cancellation_token: CancellationToken,
) -> (OptimisticBlockServiceServer<Facade>, Runner) {
    let (tx, rx) = mpsc::channel(128);

    let facade = Facade::new(tx);
    let runner = Runner::new(event_bus_subscription, rx, cancellation_token);
    let server = OptimisticBlockServiceServer::new(facade);
    (server, runner)
}

struct StartOptimisticBlockStreamRequest {
    rollup_id: RollupId,
    response: mpsc::Sender<Result<GetOptimisticBlockStreamResponse, Status>>,
}

struct StartBlockCommitmentStreamRequest {
    response: mpsc::Sender<tonic::Result<GetBlockCommitmentStreamResponse>>,
}

enum NewStreamRequest {
    OptimisticBlockStream(StartOptimisticBlockStreamRequest),
    BlockCommitmentStream(StartBlockCommitmentStreamRequest),
}

pub(super) struct Runner {
    event_bus_subscription: EventBusSubscription,
    stream_request_receiver: mpsc::Receiver<NewStreamRequest>,
    stream_tasks: JoinSet<Result<(), eyre::Report>>,
    cancellation_token: CancellationToken,
}

impl Runner {
    fn new(
        event_bus_subscription: EventBusSubscription,
        stream_request_receiver: mpsc::Receiver<NewStreamRequest>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            event_bus_subscription,
            stream_request_receiver,
            stream_tasks: JoinSet::new(),
            cancellation_token,
        }
    }

    fn handle_optimistic_block_stream_request(
        &mut self,
        request: StartOptimisticBlockStreamRequest,
    ) {
        let StartOptimisticBlockStreamRequest {
            rollup_id,
            response,
        } = request;

        let process_proposal_blocks = self.event_bus_subscription.process_proposal_blocks();
        self.stream_tasks.spawn(optimistic_stream(
            process_proposal_blocks,
            rollup_id,
            response,
            self.cancellation_token.child_token(),
        ));
    }

    fn handle_block_commitment_stream_request(
        &mut self,
        request: StartBlockCommitmentStreamRequest,
    ) {
        let StartBlockCommitmentStreamRequest {
            response,
        } = request;

        let finalized_blocks = self.event_bus_subscription.finalized_blocks();
        self.stream_tasks.spawn(block_commitment_stream(
            finalized_blocks,
            response,
            self.cancellation_token.child_token(),
        ));
    }

    pub(super) async fn run(mut self) {
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
                Some(joined_task) = self.stream_tasks.join_next() => {
                    match joined_task {
                        Ok(Ok(())) => {
                            trace!("stream task has been joined successfully");
                        },
                        Ok(Err(error)) => {
                            warn!(%error, "stream task has been joined with an error");
                        },
                        Err(error) => {
                            warn!(%error, "stream task has panicked");
                        }
                    }
                }
            }
        }

        self.shutdown().await;
    }

    #[instrument(skip_all)]
    async fn shutdown(&mut self) {
        match tokio::time::timeout(STREAM_TASKS_SHUTDOWN_DURATION, async {
            while let Some(joined_tasks) = self.stream_tasks.join_next().await {
                match joined_tasks {
                    Ok(Ok(())) => {
                        trace!("stream task has been joined successfully");
                    }
                    Ok(Err(error)) => {
                        warn!(%error, "stream task has been joined with an error");
                    }
                    Err(error) => {
                        warn!(%error, "stream task has panicked");
                    }
                }
            }
        })
        .await
        {
            Ok(()) => {
                info!("all stream tasks have been joined successfully");
            }
            Err(error) => {
                error!(%error, "stream tasks failed to shut down in time");
                self.stream_tasks.abort_all();
            }
        }
    }
}

pub(super) struct Facade {
    stream_request_sender: mpsc::Sender<NewStreamRequest>,
}

impl Facade {
    fn new(stream_request_sender: mpsc::Sender<NewStreamRequest>) -> Self {
        Self {
            stream_request_sender,
        }
    }

    #[instrument(skip_all)]
    async fn spawn_optimistic_block_stream(
        &self,
        get_optimistic_block_stream_request: GetOptimisticBlockStreamRequest,
    ) -> tonic::Result<Response<GrpcStream<GetOptimisticBlockStreamResponse>>> {
        let rollup_id = {
            let rollup_id = get_optimistic_block_stream_request
                .rollup_id
                .ok_or_else(|| Status::invalid_argument("rollup id is required"))?;

            RollupId::try_from_raw(rollup_id)
                .map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        let (tx, rx) =
            tokio::sync::mpsc::channel::<tonic::Result<GetOptimisticBlockStreamResponse>>(128);

        let request = NewStreamRequest::OptimisticBlockStream(StartOptimisticBlockStreamRequest {
            rollup_id,
            response: tx,
        });

        self.stream_request_sender
            .send(request)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to create optimistic block stream: {e}"))
            })?;

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as GrpcStream<GetOptimisticBlockStreamResponse>
        ))
    }

    #[instrument(skip_all)]
    async fn spawn_block_commitment_stream_request(
        &self,
    ) -> tonic::Result<Response<GrpcStream<GetBlockCommitmentStreamResponse>>> {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<tonic::Result<GetBlockCommitmentStreamResponse>>(128);

        let request = NewStreamRequest::BlockCommitmentStream(StartBlockCommitmentStreamRequest {
            response: tx,
        });

        self.stream_request_sender
            .send(request)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to create block commitment stream: {e}"))
            })?;

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as GrpcStream<GetBlockCommitmentStreamResponse>
        ))
    }
}

#[async_trait::async_trait]
impl OptimisticBlockService for Facade {
    type GetBlockCommitmentStreamStream = GrpcStream<GetBlockCommitmentStreamResponse>;
    type GetOptimisticBlockStreamStream = GrpcStream<GetOptimisticBlockStreamResponse>;

    #[instrument(skip_all)]
    async fn get_optimistic_block_stream(
        self: Arc<Self>,
        request: Request<GetOptimisticBlockStreamRequest>,
    ) -> tonic::Result<Response<Self::GetOptimisticBlockStreamStream>> {
        let get_optimistic_block_stream_request = request.into_inner();

        self.spawn_optimistic_block_stream(get_optimistic_block_stream_request)
            .await
    }

    #[instrument(skip_all)]
    async fn get_block_commitment_stream(
        self: Arc<Self>,
        _request: Request<GetBlockCommitmentStreamRequest>,
    ) -> tonic::Result<Response<Self::GetBlockCommitmentStreamStream>> {
        self.spawn_block_commitment_stream_request().await
    }
}

async fn block_commitment_stream(
    mut finalized_blocks_receiver: EventReceiver<Arc<FinalizeBlock>>,
    tx: mpsc::Sender<tonic::Result<GetBlockCommitmentStreamResponse>>,
    cancellation_token: CancellationToken,
) -> Result<(), eyre::Report> {
    match cancellation_token
        .run_until_cancelled(async move {
            loop {
                match finalized_blocks_receiver.receive().await {
                    Ok(finalized_block) => {
                        if let Err(error) =
                            info_span!(BLOCK_COMMITMENT_STREAM_SPAN).in_scope(|| {
                                let Hash::Sha256(block_hash) = finalized_block.hash else {
                                    warn!("block hash is empty; this should not occur");
                                    return Ok(());
                                };

                                let sequencer_block_commit = SequencerBlockCommit::new(
                                    finalized_block.height.value(),
                                    block::Hash::new(block_hash),
                                );

                                let get_block_commitment_stream_response =
                                    GetBlockCommitmentStreamResponse {
                                        commitment: Some(sequencer_block_commit.to_raw()),
                                    };

                                match tx
                                    .try_send(Ok(get_block_commitment_stream_response))
                                    .wrap_err("forwarding block commitment stream to client failed")
                                {
                                    Ok(()) => Ok(()),
                                    Err(error) => {
                                        error!(%error);
                                        Err(error)
                                    }
                                }
                            })
                        {
                            break Err(error);
                        }
                    }
                    Err(e) => {
                        break Err(e).wrap_err("failed receiving finalized block from event bus");
                    }
                }
            }
        })
        .await
    {
        Some(res) => res,
        None => Ok(()),
    }
}

async fn optimistic_stream(
    mut process_proposal_blocks: EventReceiver<Arc<SequencerBlock>>,
    rollup_id: RollupId,
    tx: mpsc::Sender<Result<GetOptimisticBlockStreamResponse, Status>>,
    cancellation_token: CancellationToken,
) -> Result<(), eyre::Report> {
    match cancellation_token
        .run_until_cancelled(async move {
            loop {
                match process_proposal_blocks.receive().await {
                    Ok(block) => {
                        if let Err(e) = info_span!(OPTIMISTIC_STREAM_SPAN).in_scope(|| {
                            let filtered_optimistic_block =
                                block.to_filtered_block(vec![rollup_id]);
                            let raw_filtered_optimistic_block =
                                filtered_optimistic_block.into_raw();

                            let get_optimistic_block_stream_response =
                                GetOptimisticBlockStreamResponse {
                                    block: Some(raw_filtered_optimistic_block),
                                };

                            match tx
                                .try_send(Ok(get_optimistic_block_stream_response))
                                .wrap_err("forwarding optimistic block stream to client failed")
                            {
                                Ok(()) => Ok(()),
                                Err(error) => {
                                    error!(%error);
                                    Err(error)
                                }
                            }
                        }) {
                            break Err(e);
                        }
                    }
                    Err(e) => {
                        break Err(e).wrap_err("failed receiving proposed block from event bus");
                    }
                }
            }
        })
        .await
    {
        Some(res) => res,
        None => Ok(()),
    }
}
