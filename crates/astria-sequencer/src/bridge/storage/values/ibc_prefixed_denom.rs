use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::primitive::v1::asset::IbcPrefixed as DomainIbcPrefixed;
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

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::bridge) struct IbcPrefixedDenom<'a>(Cow<'a, [u8; 32]>);

impl Debug for IbcPrefixedDenom<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a> From<&'a DomainIbcPrefixed> for IbcPrefixedDenom<'a> {
    fn from(ibc_prefixed: &'a DomainIbcPrefixed) -> Self {
        IbcPrefixedDenom(Cow::Borrowed(ibc_prefixed.as_bytes()))
    }
}

impl<'a> From<IbcPrefixedDenom<'a>> for DomainIbcPrefixed {
    fn from(ibc_prefixed: IbcPrefixedDenom<'a>) -> Self {
        DomainIbcPrefixed::new(ibc_prefixed.0.into_owned())
    }
}

impl<'a> From<IbcPrefixedDenom<'a>> for crate::storage::StoredValue<'a> {
    fn from(denom: IbcPrefixedDenom<'a>) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::IbcPrefixedDenom(denom)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for IbcPrefixedDenom<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::IbcPrefixedDenom(denom))) = value
        else {
            bail!(
                "bridge stored value type mismatch: expected ibc-prefixed denom, found {value:?}"
            );
        };
        Ok(denom)
    }
}
