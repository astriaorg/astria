//! The allocation rule is the mechanism by which the auction processes incoming bids and determines
//! the winner.
use super::Bundle;

pub(super) struct FirstPrice {
    highest_bid: Option<Bundle>,
}

impl FirstPrice {
    pub(super) fn new() -> Self {
        Self {
            highest_bid: None,
        }
    }

    /// Submit a bundle with a bid.
    ///
    /// Returns `true` if the bid is accepted as the highest bid.
    pub(super) fn bid(&mut self, bundle: Bundle) -> bool {
        if bundle.bid() > self.highest_bid.as_ref().map_or(0, Bundle::bid) {
            self.highest_bid = Some(bundle);
            true
        } else {
            false
        }
    }

    /// Returns the winner of the auction, if one exists.
    pub(super) fn winner(self) -> Option<Bundle> {
        self.highest_bid
    }
}
