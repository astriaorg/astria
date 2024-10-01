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

use crate::accounts::AddressBytes as DomainAddressBytes;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    Balance(Balance),
    AddressBytes(AddressBytes<'a>),
    Fee(Fee),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::ibc) struct Balance(u128);

impl From<u128> for Balance {
    fn from(balance: u128) -> Self {
        Balance(balance)
    }
}

impl From<Balance> for u128 {
    fn from(balance: Balance) -> Self {
        balance.0
    }
}

impl<'a> From<Balance> for crate::storage::StoredValue<'a> {
    fn from(balance: Balance) -> Self {
        crate::storage::StoredValue::Ibc(Value(ValueImpl::Balance(balance)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Balance {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value(ValueImpl::Balance(balance))) = value else {
            bail!("ibc stored value type mismatch: expected balance, found {value:?}");
        };
        Ok(balance)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::ibc) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

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
        crate::storage::StoredValue::Ibc(Value(ValueImpl::AddressBytes(address)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressBytes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value(ValueImpl::AddressBytes(address))) = value
        else {
            bail!("ibc stored value type mismatch: expected address bytes, found {value:?}");
        };
        Ok(address)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::ibc) struct Fee(u128);

impl From<u128> for Fee {
    fn from(fee: u128) -> Self {
        Fee(fee)
    }
}

impl From<Fee> for u128 {
    fn from(fee: Fee) -> Self {
        fee.0
    }
}

impl<'a> From<Fee> for crate::storage::StoredValue<'a> {
    fn from(fee: Fee) -> Self {
        crate::storage::StoredValue::Ibc(Value(ValueImpl::Fee(fee)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Fee {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value(ValueImpl::Fee(fee))) = value else {
            bail!("ibc stored value type mismatch: expected fee, found {value:?}");
        };
        Ok(fee)
    }
}
