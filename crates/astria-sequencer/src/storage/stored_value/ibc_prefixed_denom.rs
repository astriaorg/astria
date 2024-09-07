use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed as DomainIbcPrefixed;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct IbcPrefixedDenom<'a>(Cow<'a, [u8; 32]>);

impl<'a> From<&'a DomainIbcPrefixed> for IbcPrefixedDenom<'a> {
    fn from(ibc_prefixed: &'a DomainIbcPrefixed) -> Self {
        IbcPrefixedDenom(Cow::Borrowed(ibc_prefixed.get()))
    }
}

impl<'a> From<IbcPrefixedDenom<'a>> for DomainIbcPrefixed {
    fn from(ibc_prefixed: IbcPrefixedDenom<'a>) -> Self {
        DomainIbcPrefixed::new(ibc_prefixed.0.into_owned())
    }
}

impl<'a> TryFrom<StoredValue<'a>> for IbcPrefixedDenom<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::IbcPrefixedDenom(denom) = value else {
            return Err(super::type_mismatch("ibc-prefixed denom", &value));
        };
        Ok(denom)
    }
}
