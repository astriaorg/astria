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

    pub(crate) fn bid(&mut self, bundle: Bundle) -> bool {
        if bundle.bid() > self.highest_bid.as_ref().map_or(0, |b| b.bid()) {
            self.highest_bid = Some(bundle);
            true
        } else {
            false
        }
    }

    pub(crate) fn highest_bid(self) -> Option<Bundle> {
        self.highest_bid
    }
}
