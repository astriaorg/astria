use astria_core::protocol::transaction::v1alpha1::SignedTransaction;

struct Bundle;

pub(crate) struct Bid {
    fee: u64,
    bundle: Bundle,
}

pub(crate) enum _State {
    Closed,
    Open,
    Timer,
    Result,
}

pub(crate) struct _FirstPriceAuction {
    latency_margin: u64,
}

impl _FirstPriceAuction {
    // TODO:
    // 1. add bid
    // 2. start timer
    // 3. get result
}

#[derive(Hash, Eq, PartialEq)]
pub(crate) struct Id {
    pub(crate) sequencer_block_hash: String,
}

pub(crate) struct Winner {
    _submitted_transaction: SignedTransaction,
}
