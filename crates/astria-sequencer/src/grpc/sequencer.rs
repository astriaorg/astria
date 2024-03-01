use anyhow::Context as _;
use astria_core::{
    generated::sequencer::v1alpha1::{
        sequencer_service_server::SequencerService,
        FilteredSequencerBlock as RawFilteredSequencerBlock,
        FilteredSequencerBlockRequest,
        GetSequencerBlockRequest,
        SequencerBlock as RawSequencerBlock,
    },
    sequencer::v1alpha1::RollupId,
};
use cnidarium::Storage;
use sequencer_client::{
    HttpClient,
    SequencerClientExt as _,
};
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::instrument;

use crate::state_ext::StateReadExt as _;

pub(crate) struct SequencerServer {
    client: HttpClient,
    storage: Storage,
}

impl SequencerServer {
    pub(crate) fn new(cometbft_endpoint: &str, storage: Storage) -> anyhow::Result<Self> {
        let client = HttpClient::new(cometbft_endpoint)
            .context("should be able to create cometbft client")?;
        Ok(Self {
            client,
            storage,
        })
    }
}

#[async_trait::async_trait]
impl SequencerService for SequencerServer {
    /// Given a block height, returns the sequencer block at that height.
    #[instrument(skip_all, fields(height = request.get_ref().height))]
    async fn get_sequencer_block(
        &self,
        request: Request<GetSequencerBlockRequest>,
    ) -> Result<Response<RawSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let height: u32 = request
            .height
            .try_into()
            .map_err(|_| Status::invalid_argument("height should be a valid u32"))?;

        let block = match self.client.sequencer_block(height).await {
            Ok(block) => block.into_raw(),
            Err(_) => {
                return Err(Status::internal(
                    "failed to get sequencer block from cometbft",
                ));
            }
        };

        Ok(Response::new(block))
    }

    /// Given a block height and set of rollup ids, returns a SequencerBlock which
    /// is filtered to contain only the transactions that are relevant to the given rollup.
    #[instrument(skip_all, fields(height = request.get_ref().height))]
    async fn get_filtered_sequencer_block(
        &self,
        request: Request<FilteredSequencerBlockRequest>,
    ) -> Result<Response<RawFilteredSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let height: u32 = request
            .height
            .try_into()
            .map_err(|_| Status::invalid_argument("height should be a valid u32"))?;

        let mut rollup_ids: Vec<RollupId> = vec![];
        for id in request.rollup_ids {
            let Ok(rollup_id) = RollupId::try_from_vec(id) else {
                return Err(Status::invalid_argument(
                    "invalid rollup ID; must be 32 bytes",
                ));
            };
            rollup_ids.push(rollup_id);
        }

        let block = match self.client.sequencer_block(height).await {
            Ok(block) => block.into_filtered_block(rollup_ids),
            Err(e) => {
                return Err(Status::internal(format!(
                    "failed to get sequencer block from cometbft: {e}",
                )));
            }
        };

        Ok(Response::new(block.into_raw()))
    }
}
