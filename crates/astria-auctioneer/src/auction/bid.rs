use astria_core::{
    generated::bundle::v1alpha1 as raw,
    protocol::transaction::v1alpha1::SignedTransaction,
};
use astria_eyre::eyre::{
    self,
};

// TODO: this should probably be moved to astria_core::bundle
pub(crate) struct Bundle {}

impl Bundle {
    fn try_from_raw(raw: raw::Bundle) -> eyre::Result<Self> {
        unimplemented!()
    }

    fn into_raw(self) -> raw::Bundle {
        unimplemented!()
    }

    pub(crate) fn into_transaction(self) -> SignedTransaction {
        unimplemented!()
    }
}

pub(crate) struct Bid {}

impl Bid {
    fn from_bundle(bundle: Bundle) -> Self {
        unimplemented!()
    }

    fn into_bundle(self) -> Bundle {
        unimplemented!()
    }
}
