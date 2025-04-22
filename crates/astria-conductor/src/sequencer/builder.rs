use std::time::Duration;

use astria_core::sequencerblock::v1alpha1::block::FilteredSequencerBlock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::SequencerGrpcClient;
use crate::state::StateReceiver;

pub(crate) struct Builder {
    pub(crate) sequencer_grpc_client: SequencerGrpcClient,
    pub(crate) sequencer_cometbft_client: sequencer_client::HttpClient,
    pub(crate) sequencer_block_time: Duration,
    pub(crate) shutdown: CancellationToken,
    pub(crate) rollup_state: StateReceiver,
    pub(crate) soft_blocks: mpsc::Sender<FilteredSequencerBlock>,
}

impl Builder {
    pub(crate) fn build(self) -> super::Reader {
        let Self {
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
            rollup_state,
            soft_blocks,
        } = self;
        super::Reader {
            rollup_state,
            soft_blocks,
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
        }
    }
}
