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
    pub(super) fn bid(&mut self, candidate: Bundle) -> bool {
        if let Some(current) = self.highest_bid.as_mut() {
            let is_higher = candidate.bid() > current.bid();
            if is_higher {
                *current = candidate;
            }
            is_higher
        } else {
            self.highest_bid = Some(candidate);
            true
        }
    }

    /// Returns the winner of the auction, if one exists.
    pub(super) fn winner(self) -> Option<Bundle> {
        self.highest_bid
    }
}
