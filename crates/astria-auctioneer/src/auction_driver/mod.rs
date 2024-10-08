use astria_eyre::eyre;
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_util::sync::CancellationToken;

use crate::{
    auction,
    Metrics,
};

mod builder;
pub(crate) use builder::Builder;

pub(crate) struct Handle {
    executed_block_tx: Option<oneshot::Sender<()>>,
    block_commitments_tx: Option<oneshot::Sender<()>>,
    reorg_tx: Option<oneshot::Sender<()>>,
    new_bids_tx: mpsc::Sender<auction::Bid>,
}

impl Handle {
    pub(crate) async fn send_bundle(&self, bundle: auction::Bid) -> eyre::Result<()> {
        self.new_bids_tx.send(bundle).await?;
        Ok(())
    }

    pub(crate) fn executed_block(&mut self) -> eyre::Result<()> {
        let tx = self
            .executed_block_tx
            .take()
            .expect("should only send executed signal to auction once per block");
        let _ = tx.send(());
        Ok(())
    }
}

pub(crate) struct AuctionDriver {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    shutdown_token: CancellationToken,
    // TODO:
    // - The current block being used to drive the [`Auction`]
    // - The current [`Auction`] being driven
    // - Submitter
    // oneshot channels for:
    // - start the timer
    // - graceful shutdown
    executed_block_rx: oneshot::Receiver<()>,
    block_commitments_rx: oneshot::Receiver<()>,
    reorg_rx: oneshot::Receiver<()>,
    new_bids_rx: mpsc::Receiver<auction::Bid>,
}

impl AuctionDriver {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            ..
        } = self;

        // initialize this when the block is executed
        //
        // loop over:
        //  1. result from submit_fut if !submission.terminated()
        //  2. shutdown signal for early termination (e.g. due to reorg)
        //  3. timer expires if submission.terminated()
        //      - submit_fut = new_submit_fut(auction.result())
        //      - this consumes the auction, setting it to none?
        //  4. commit signal arrives if auction exists?
        //      - start the timer
        //  5. executed signal arrives if auction exists?
        //      - set the flag to start processing bundles
        //  6. new bundles from the bundle stream if auction exists?
        //      - add the bid to the auction if executed
        //
        // 3-6 should be methods on the auction object that manages auction state (init, open,
        // closing, result):
        // - start at init
        // - open happens when executed arrives, auction starts processing bids
        // - the timer starts when commit arrives
        // - result happens when the timer expires and the action is consumed to be submitted. this
        //   stops the processing of bids as it consumes the auction
        //
        // the auction object produces result of the auction to consume the object, and that result
        // can be put into a sequencer tx and given to the submit_fut.
        //
        // return the submission result
        Ok(())
    }
}
