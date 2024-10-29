use std::{
    pin::Pin,
    sync::Arc,
};

use astria_core::{
    generated::sequencerblock::v1alpha1::{
        optimistic_block_service_server::OptimisticBlockService,
        GetBlockCommitmentStreamRequest,
        GetBlockCommitmentStreamResponse,
        GetOptimisticBlockStreamRequest,
        GetOptimisticBlockStreamResponse,
    },
    primitive::v1::RollupId,
    Protobuf,
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
    info,
};

use crate::app::OptimisticBlockChannels;

pub(crate) struct OptimisticBlockServer {
    optimistic_block_channels: OptimisticBlockChannels,
}

impl OptimisticBlockServer {
    pub(crate) fn new(optimistic_block_channels: OptimisticBlockChannels) -> Self {
        Self {
            optimistic_block_channels,
        }
    }
}

#[async_trait::async_trait]
impl OptimisticBlockService for OptimisticBlockServer {
    type GetBlockCommitmentStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetBlockCommitmentStreamResponse, Status>> + Send>>;
    type GetOptimisticBlockStreamStream =
        Pin<Box<dyn Stream<Item = Result<GetOptimisticBlockStreamResponse, Status>> + Send>>;

    async fn get_optimistic_block_stream(
        self: Arc<Self>,
        request: Request<GetOptimisticBlockStreamRequest>,
    ) -> Result<Response<Self::GetOptimisticBlockStreamStream>, Status> {
        let get_optimistic_block_stream_request = request.into_inner();

        let rollup_id = {
            let rollup_id = if let Some(rollup_id) = get_optimistic_block_stream_request.rollup_id {
                rollup_id
            } else {
                return Err(Status::invalid_argument("rollup id is required"));
            };

            match RollupId::try_from_raw(rollup_id) {
                Ok(rollup_id) => rollup_id,
                Err(_) => {
                    return Err(Status::invalid_argument("invalid rollup id"));
                }
            }
        };

        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<GetOptimisticBlockStreamResponse, Status>>(128);

        let mut optimistic_block_receiver = self
            .optimistic_block_channels
            .optimistic_block_sender()
            .subscribe();

        tokio::spawn(async move {
            loop {
                while let Ok(()) = optimistic_block_receiver.changed().await {
                    let optimistic_block = optimistic_block_receiver
                        .borrow_and_update()
                        .clone()
                        .expect("received block is none");

                    let filtered_optimistic_block =
                        optimistic_block.to_filtered_block(vec![rollup_id]);
                    let raw_filtered_optimistic_block = filtered_optimistic_block.into_raw();

                    let get_optimistic_block_stream_response = GetOptimisticBlockStreamResponse {
                        block: Some(raw_filtered_optimistic_block),
                    };

                    match tx
                        .send(Result::<_, Status>::Ok(
                            get_optimistic_block_stream_response,
                        ))
                        .await
                    {
                        Ok(()) => {
                            debug!("sent optimistic block");
                        }
                        Err(_item) => {
                            info!("receiver dropped");
                            break;
                        }
                    };
                }
                debug!("optimistic block receiver changed failed");
            }
        });

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

        let mut committed_block_receiver = self
            .optimistic_block_channels
            .committed_block_sender()
            .subscribe();

        tokio::spawn(async move {
            loop {
                while let Ok(()) = committed_block_receiver.changed().await {
                    let sequencer_block_commit = committed_block_receiver
                        .borrow_and_update()
                        .clone()
                        .expect("received block is none");

                    let get_block_commitment_stream_response = GetBlockCommitmentStreamResponse {
                        commitment: Some(sequencer_block_commit.to_raw()),
                    };

                    match tx
                        .send(Result::<_, Status>::Ok(
                            get_block_commitment_stream_response,
                        ))
                        .await
                    {
                        Ok(()) => {
                            debug!("sent block commitment");
                        }
                        Err(_item) => {
                            info!("receiver dropped");
                            break;
                        }
                    };
                }
                debug!("commited block receiver changed failed");
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetBlockCommitmentStreamStream
        ))
    }
}
