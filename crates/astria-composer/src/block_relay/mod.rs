use color_eyre::eyre;
use sequencer_client::{
    Address,
    SignedTransaction,
};

use self::server::ProposerSubmission;

mod client;
mod server;

pub(super) struct BlockRelay {
    proposer_submission: ProposerSubmission,
}

impl BlockRelay {
    pub fn from_config() -> Self {
        todo!("initialize the block relay service");
        let (best_bid_tx, best_bid_rx) = mpsc::channel(1);
        let (committed_bundle_tx, committed_bundle_rx) = mpsc::channel(1);
        let proposer_submission =
            ProposerSubmission::new(addr, best_bid_tx, committed_bundle_tx).await?;

        Self {
            proposer_submission,
        }
    }

    pub async fn run_until_stopped(self) -> eyre::Result<()> {
        todo!("run the block relay service");
        let Self {
            proposer_submission,
        } = self;

        let proposer = tokio::spawn(proposer_submission.run_until_stopped());

        // select loop over channel sender and receiver from proposer actor
        // push constant bid into proposer
        // print out committed bundles

        Ok(())
    }
}
