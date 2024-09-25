use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
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
pub(crate) enum Value<'a> {
    Balance(Balance),
    AddressBytes(AddressBytes<'a>),
    Fee(Fee),
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Balance(balance) => write!(f, "balance {}", balance.0),
            Value::AddressBytes(address_bytes) => {
                write!(f, "address bytes {}", base64(address_bytes.0.as_slice()))
            }
            Value::Fee(fee) => write!(f, "fee {}", fee.0),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Balance(u128);

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
        crate::storage::StoredValue::Ibc(Value::Balance(balance))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Balance {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value::Balance(balance)) = value else {
            bail!("ibc stored value type mismatch: expected balance, found {value}");
        };
        Ok(balance)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

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
        crate::storage::StoredValue::Ibc(Value::AddressBytes(address))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressBytes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value::AddressBytes(address)) = value else {
            bail!("ibc stored value type mismatch: expected address bytes, found {value}");
        };
        Ok(address)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Fee(u128);

impl Display for Fee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
        crate::storage::StoredValue::Ibc(Value::Fee(fee))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Fee {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value::Fee(fee)) = value else {
            bail!("ibc stored value type mismatch: expected fee, found {value}");
        };
        Ok(fee)
    }
}
