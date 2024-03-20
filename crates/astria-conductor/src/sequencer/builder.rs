use std::time::Duration;

use tokio::sync::oneshot;

use super::SequencerGrpcClient;
use crate::executor;

pub(crate) struct Builder {
    pub(crate) executor: executor::Handle,
    pub(crate) sequencer_grpc_client: SequencerGrpcClient,
    pub(crate) sequencer_cometbft_client: sequencer_client::HttpClient,
    pub(crate) sequencer_block_time: Duration,
    pub(crate) shutdown: oneshot::Receiver<()>,
}

impl Builder {
    pub(crate) fn build(self) -> super::Reader {
        let Self {
            executor,
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
        } = self;
        super::Reader {
            executor,
            sequencer_grpc_client,
            sequencer_cometbft_client,
            sequencer_block_time,
            shutdown,
        }
    }
}
