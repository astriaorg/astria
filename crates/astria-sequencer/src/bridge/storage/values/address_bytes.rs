use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::primitive::v1::ADDRESS_LEN;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};
use crate::accounts::AddressBytes as DomainAddressBytes;

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::bridge) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

impl<'a> Debug for AddressBytes<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a, T: DomainAddressBytes> From<&'a T> for AddressBytes<'a> {
    fn from(value: &'a T) -> Self {
        AddressBytes(Cow::Borrowed(value.address_bytes()))
    }
}

impl<'a> From<AddressBytes<'a>> for [u8; ADDRESS_LEN] {
    fn from(address_bytes: AddressBytes<'a>) -> Self {
        address_bytes.0.into_owned()
    }
}

impl<'a> From<AddressBytes<'a>> for crate::storage::StoredValue<'a> {
    fn from(address: AddressBytes<'a>) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::AddressBytes(address)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressBytes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::AddressBytes(address))) = value
        else {
            bail!("bridge stored value type mismatch: expected address bytes, found {value:?}");
        };
        Ok(address)
    }
}
