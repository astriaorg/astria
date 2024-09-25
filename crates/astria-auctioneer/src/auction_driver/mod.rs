use astria_eyre::eyre;
use tokio::select;

use crate::{
    auction,
    block::{
        self,
        Block,
    },
    optimistic_executor::{
        self,
    },
    Metrics,
};

mod builder;
pub(crate) use builder::Builder;

pub(crate) struct AuctionDriver {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    /// The current block being used to drive the [`Auction`]
    curr_block: Block,
    /// The current [`Auction`] being driven
    auction: auction::FirstPriceAuction,
    // TODO: submitter
}

impl AuctionDriver {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            metrics,
            curr_block,
            auction,
        } = self;

        loop {
            select! {
                biased;

                // TODO: should this be conditioned on the block state not being committed? or the auction state not be "closing"?
                // instead of advancing the block state here i should have a handle that reads current state from an arc?
                curr_block = curr_block.next_state(), if auction.committed() => {
                    match curr_block.state() {
                        block::State::OptimisticBlock(optimistic_block) => {
                            todo!("drop old auction");
                            todo!("make new auction");
                        },
                        block::State::ExecutedBlock(executed_block) => todo!(),
                        block::State::BlockCommitment(block_commitment) => todo!(),
                        block::State::Reorg(reorg_block) => todo!(),
                    }
                },

                // TODO: submit when auction is ready for submission
                //
            }
        }

        Ok(())
    }
}
