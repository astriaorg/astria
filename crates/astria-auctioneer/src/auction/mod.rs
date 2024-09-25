use crate::block;

mod bid;
mod builder;
mod driver;
use bid::Bundle;
pub(crate) use driver::Handle;

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) struct Id([u8; 32]);

impl Id {
    fn from_optimistic_block(optimistic_block: block::Optimistic) -> Self {
        Self(optimistic_block.sequencer_block_hash())
    }
}

struct Auction {
    highest_bid: Option<Bundle>,
}

impl Auction {
    fn new() -> Self {
        Self {
            highest_bid: None,
        }
    }

    fn bid(&mut self, bid: Bundle) -> bool {
        // save the bid if its higher than self.highest_bid
        unimplemented!()
    }

    fn winner(self) -> Bundle {
        unimplemented!()
    }
}
