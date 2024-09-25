use astria_core::{
    generated::bundle::v1alpha1 as raw,
    protocol::transaction::v1::Transaction,
};
use astria_eyre::eyre::{
    self,
};
use bytes::Bytes;

// TODO: this should probably be moved to astria_core::bundle
#[derive(Debug, Clone)]
pub(crate) struct Bundle {
    raw: raw::Bundle,
    /// The fee that will be charged for this bundle
    fee: u64,
    /// The byte list of transactions fto be included.
    transactions: Vec<Bytes>,
    /// The hash of the rollup block that this bundle is based on.
    prev_rollup_block_hash: Bytes,
    /// The hash of the sequencer block used to derive the rollup block that this bundle is based
    /// on.
    base_sequencer_block_hash: Bytes,
}

impl Bundle {
    fn try_from_raw(_raw: raw::Bundle) -> eyre::Result<Self> {
        unimplemented!()
        // Ok(Self {
        //     raw,
        // })
    }

    fn into_raw(self) -> raw::Bundle {
        unimplemented!()
    }

    pub(crate) fn into_transaction(self, _nonce: u64) -> Transaction {
        unimplemented!()
    }

    pub(crate) fn into_bid(self) -> Bid {
        Bid::from_bundle(self)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Bid {}

impl Bid {
    fn from_bundle(_bundle: Bundle) -> Self {
        unimplemented!()
    }

    fn into_bundle(self) -> Bundle {
        unimplemented!()
    }
}
