use crate::block;

mod bid;
mod builder;
mod driver;
use bid::Bundle;
pub(crate) use driver::Handle;

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) struct Id([u8; 32]);

impl Id {
    pub(crate) fn from_sequencer_block_hash(block_hash: [u8; 32]) -> Self {
        Self(block_hash)
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
