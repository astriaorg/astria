use std::borrow::Cow;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct AddressPrefix<'a>(Cow<'a, str>);

impl<'a> From<&'a str> for AddressPrefix<'a> {
    fn from(address_prefix: &'a str) -> Self {
        AddressPrefix(Cow::Borrowed(address_prefix))
    }
}

impl<'a> From<AddressPrefix<'a>> for String {
    fn from(address_prefix: AddressPrefix<'a>) -> Self {
        address_prefix.0.into_owned()
    }
}

impl<'a> TryFrom<StoredValue<'a>> for AddressPrefix<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::AddressPrefix(address_prefix) = value else {
            return Err(super::type_mismatch("address prefix", &value));
        };
        Ok(address_prefix)
    }
}
