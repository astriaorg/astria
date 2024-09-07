use std::borrow::Cow;

use astria_core::primitive::v1::ADDRESS_LEN;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;
use crate::accounts::AddressBytes as DomainAddressBytes;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

impl<'a, T: DomainAddressBytes> From<&'a T> for AddressBytes<'a> {
    fn from(value: &'a T) -> Self {
        AddressBytes(Cow::Borrowed(value.address_bytes()))
    }
}

impl<'a> From<&'a tendermint::account::Id> for AddressBytes<'a> {
    fn from(account_id: &'a tendermint::account::Id) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(ADDRESS_LEN == tendermint::account::LENGTH);
        let mut proposer_address_bytes = [0; ADDRESS_LEN];
        proposer_address_bytes.copy_from_slice(account_id.as_bytes());
        AddressBytes(Cow::Owned(proposer_address_bytes))
    }
}

impl<'a> From<AddressBytes<'a>> for [u8; ADDRESS_LEN] {
    fn from(address_bytes: AddressBytes<'a>) -> Self {
        address_bytes.0.into_owned()
    }
}

impl<'a> TryFrom<StoredValue<'a>> for AddressBytes<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::AddressBytes(address) = value else {
            return Err(super::type_mismatch("address bytes", &value));
        };
        Ok(address)
    }
}
