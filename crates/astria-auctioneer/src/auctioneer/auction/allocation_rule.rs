//! The allocation rule is the mechanism by which the auction processes incoming bids and determines
//! the winner.
use std::sync::Arc;

use tracing::{
    info,
    instrument,
};

use super::Bid;

pub(super) struct FirstPrice {
    highest_bid: Option<Arc<Bid>>,
    bids_seen: usize,
}

impl FirstPrice {
    pub(super) fn new() -> Self {
        Self {
            highest_bid: None,
            bids_seen: 0,
        }
    }

    /// Submit a bid with a bid.
    ///
    /// Returns `true` if the bid is accepted as the highest bid.
    // TODO: identify the incumbant and candidate by their hash?
    #[instrument(skip_all, fields(
        current_winner.bid = self.highest_bid.as_ref().map(|bid| bid.bid()),
        candidate.bid = candidate.bid(),
    ))]
    pub(super) fn bid(&mut self, candidate: &Arc<Bid>) {
        self.bids_seen = self.bids_seen.saturating_add(1);
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

    pub(super) fn bids_seen(&self) -> usize {
        self.bids_seen
    }

    /// Returns the winner of the auction, if one exists.
    pub(super) fn take_winner(&mut self) -> Option<Arc<Bid>> {
        self.highest_bid.take()
    }
}
