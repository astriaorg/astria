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
}

impl AuctionDriver {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            ..
        } = self;

        loop {
            // select! {
            //     biased;

            // TODO: should this be conditioned on the block state not being committed? or the
            // auction state not be "closing"? instead of advancing the block state here
            // i should have a handle that reads current state from an arc?
            // curr_block =
            //  curr_block.apply_state(), if !auction.committed() => {
            //      match curr_block.state() {
            //          optimistic ->
            //           drop old auction if it exists
            //           make new auction
            //          executed -> open the auction for bids
            //          committed -> start the timer for closing
            //          committed and executed -> ?
            //      },
            //  }
            // },

            // new bundle from the bundle stream -> if auction is open, add the bid to the auction.
            // otherwise log and drop

            // TODO: submit when auction is ready for submission
            //
            // }
            break;
        }

        Ok(())
    }
}
