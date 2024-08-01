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

pub(crate) trait GetAddressBytes: Send + Sync {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN];
}

impl GetAddressBytes for Address {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.bytes()
    }
}

impl GetAddressBytes for [u8; ADDRESS_LEN] {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        *self
    }
}

impl GetAddressBytes for SignedTransaction {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl GetAddressBytes for SigningKey {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl GetAddressBytes for VerificationKey {
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes()
    }
}

impl<'a, T> GetAddressBytes for &'a T
where
    T: GetAddressBytes,
{
    fn get_address_bytes(&self) -> [u8; ADDRESS_LEN] {
        (*self).get_address_bytes()
    }
}
