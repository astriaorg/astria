use std::borrow::Cow;

use astria_core::primitive::v1::asset::TracePrefixed as DomainTracePrefixed;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct TracePrefixedDenom<'a> {
    trace: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    base_denom: Cow<'a, str>,
}

impl<'a> From<&'a DomainTracePrefixed> for TracePrefixedDenom<'a> {
    fn from(trace_prefixed: &'a DomainTracePrefixed) -> Self {
        TracePrefixedDenom {
            trace: trace_prefixed
                .trace()
                .map(|(port, channel)| (Cow::Borrowed(port), Cow::Borrowed(channel)))
                .collect(),
            base_denom: Cow::Borrowed(trace_prefixed.base_denom()),
        }
    }
}

impl<'a> From<TracePrefixedDenom<'a>> for DomainTracePrefixed {
    fn from(trace_prefixed: TracePrefixedDenom<'a>) -> Self {
        DomainTracePrefixed::unchecked_from_parts(
            trace_prefixed
                .trace
                .into_iter()
                .map(|(port, channel)| (port.into_owned(), channel.into_owned())),
            trace_prefixed.base_denom.into_owned(),
        )
    }
}

impl<'a> TryFrom<StoredValue<'a>> for TracePrefixedDenom<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::TracePrefixedDenom(denom) = value else {
            return Err(super::type_mismatch("trace-prefixed denom", &value));
        };
        Ok(denom)
    }
}
