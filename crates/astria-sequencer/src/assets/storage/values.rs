use std::borrow::Cow;

use astria_core::primitive::v1::asset::TracePrefixed as DomainTracePrefixed;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    TracePrefixedDenom(TracePrefixedDenom<'a>),
    Fee(Fee),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::assets) struct TracePrefixedDenom<'a> {
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

impl<'a> From<TracePrefixedDenom<'a>> for crate::storage::StoredValue<'a> {
    fn from(denom: TracePrefixedDenom<'a>) -> Self {
        crate::storage::StoredValue::Assets(Value(ValueImpl::TracePrefixedDenom(denom)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for TracePrefixedDenom<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Assets(Value(ValueImpl::TracePrefixedDenom(denom))) =
            value
        else {
            bail!(
                "assets stored value type mismatch: expected trace-prefixed denom, found {value:?}"
            );
        };
        Ok(denom)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::assets) struct Fee(u128);

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
        crate::storage::StoredValue::Assets(Value(ValueImpl::Fee(fee)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Fee {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Assets(Value(ValueImpl::Fee(fee))) = value else {
            bail!("assets stored value type mismatch: expected fee, found {value:?}");
        };
        Ok(fee)
    }
}
