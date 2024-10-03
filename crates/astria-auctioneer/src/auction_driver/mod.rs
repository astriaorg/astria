use astria_eyre::eyre;

use crate::Metrics;

mod builder;
pub(crate) use builder::Builder;

pub(crate) struct AuctionDriver {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    // TODO:
    // - The current block being used to drive the [`Auction`]
    // - The current [`Auction`] being driven
    // - Submitter
    // oneshot channels for:
    // - start the timer
    // - graceful shutdown
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
