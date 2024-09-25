struct Bundle;

pub(crate) struct Bid {
    fee: u64,
    bundle: Bundle,
}

pub(crate) enum State {
    Closed,
    Open,
    Timer,
    Result,
}

pub(crate) struct FirstPriceAuction {
    state: State,
    latency_margin: u64,
}

impl FirstPriceAuction {
    pub(crate) fn committed(&self) -> bool {
        todo!("return whether the timer has been activated")
    }

    pub(crate) fn new_bid() {}
}
