pub(crate) mod action;
pub(crate) mod component;
pub(crate) mod query;
mod state_ext;

use astria_core::{
    crypto::{
        SigningKey,
        VerificationKey,
    },
    primitive::v1::{
        Address,
        ADDRESS_LEN,
    },
    protocol::transaction::v1alpha1::SignedTransaction,
};
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

pub(crate) trait AddressBytes: Send + Sync {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN];
}

impl AddressBytes for Address {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.bytes()
    }
}

impl AddressBytes for [u8; ADDRESS_LEN] {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        *self
    }
}

impl AddressBytes for SignedTransaction {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl AddressBytes for SigningKey {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl AddressBytes for VerificationKey {
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl<'a, T> AddressBytes for &'a T
where
    T: AddressBytes,
{
    fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        (*self).address_bytes()
    }
}
