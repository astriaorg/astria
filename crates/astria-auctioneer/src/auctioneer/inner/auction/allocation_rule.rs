//! The allocation rule is the mechanism by which the auction processes incoming bids and determines
//! the winner.
use std::sync::Arc;

use tracing::{
    info,
    instrument,
};

use super::Bundle;

pub(super) struct FirstPrice {
    highest_bid: Option<Arc<Bundle>>,
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
    #[instrument(skip_all, fields(
        current_winner.bid = self.highest_bid.as_ref().map(|bundle| bundle.bid()),
        candidate.bid = candidate.bid(),
    ))]
    pub(super) fn bid(&mut self, candidate: &Arc<Bundle>) {
        let winner = if let Some(current) = self.highest_bid.as_mut() {
            if candidate.bid() > current.bid() {
                *current = candidate.clone();
                "candidate"
            } else {
                "incumbant"
            }
        } else {
            self.highest_bid = Some(candidate.clone());
            "candidate"
        };
        info!("highest bidder is {winner}");
    }

    /// Returns the winner of the auction, if one exists.
    pub(super) fn winner(self) -> Option<Arc<Bundle>> {
        self.highest_bid
    }
}
