pub(crate) mod action;
pub(crate) mod component;
pub(crate) mod query;
mod state_ext;
pub(crate) mod storage;

use astria_core::{
    crypto::VerificationKey,
    primitive::v1::{
        Address,
        ADDRESS_LENGTH,
    },
    protocol::transaction::v1::Transaction,
};
pub(crate) use state_ext::{
    AssetBalance,
    StateReadExt,
    StateWriteExt,
};

pub(crate) trait AddressBytes: Send + Sync {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH];

    fn display_address(&self) -> impl std::fmt::Display {
        telemetry::display::base64(self.address_bytes())
    }
}

impl AddressBytes for Address {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        self.as_bytes()
    }

    fn display_address(&self) -> impl std::fmt::Display {
        self
    }
}

impl AddressBytes for [u8; ADDRESS_LENGTH] {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        self
    }
}

impl<'a> AddressBytes for &'a Transaction {
    fn address_bytes(&self) -> &'a [u8; ADDRESS_LENGTH] {
        Transaction::address_bytes(self)
    }
}

impl AddressBytes for VerificationKey {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        self.address_bytes()
    }
}
