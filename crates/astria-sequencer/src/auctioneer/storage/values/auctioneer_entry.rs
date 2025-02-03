use std::borrow::Cow;

use astria_core::{
    primitive::v1::{
        asset::{
            Denom as DomainDenom,
            TracePrefixed as DomainTracePrefixed,
        },
        Address as DomainAddress,
    },
    protocol::auctioneer::v1::EnshrinedAuctioneerEntry as DomainEnshrinedAuctioneerEntry,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    AddressBytes,
    IbcPrefixedDenom,
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::auctioneer) struct Address<'a> {
    bytes: AddressBytes<'a>,
    prefix: Cow<'a, str>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct TracePrefixedDenom<'a> {
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

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::auctioneer) enum Denom<'a> {
    TracePrefixed(TracePrefixedDenom<'a>),
    IbcPrefixed(IbcPrefixedDenom<'a>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::auctioneer) struct EnshrinedAuctioneerEntry<'a> {
    pub auctioneer_address: Address<'a>,
    pub staker_address: Address<'a>,
    pub staked_amount: u128,
    pub fee_asset: Denom<'a>,
    pub asset: Denom<'a>,
}

impl<'a> From<&'a DomainEnshrinedAuctioneerEntry> for EnshrinedAuctioneerEntry<'a> {
    fn from(enshrined_auctioneer_entry: &'a DomainEnshrinedAuctioneerEntry) -> Self {
        let auctioneer_address = Address {
            bytes: enshrined_auctioneer_entry
                .auctioneer_address
                .as_bytes()
                .into(),
            prefix: Cow::Borrowed(enshrined_auctioneer_entry.auctioneer_address.prefix()),
        };
        let staker_address = Address {
            bytes: enshrined_auctioneer_entry.staker_address.as_bytes().into(),
            prefix: Cow::Borrowed(enshrined_auctioneer_entry.staker_address.prefix()),
        };

        let asset = match &enshrined_auctioneer_entry.asset {
            DomainDenom::TracePrefixed(denom) => Denom::TracePrefixed(denom.into()),
            DomainDenom::IbcPrefixed(denom) => Denom::IbcPrefixed(denom.into()),
        };

        let fee_asset = match &enshrined_auctioneer_entry.fee_asset {
            DomainDenom::TracePrefixed(denom) => Denom::TracePrefixed(denom.into()),
            DomainDenom::IbcPrefixed(denom) => Denom::IbcPrefixed(denom.into()),
        };

        EnshrinedAuctioneerEntry {
            auctioneer_address,
            staker_address,
            staked_amount: enshrined_auctioneer_entry.staked_amount,
            asset,
            fee_asset,
        }
    }
}

impl<'a> From<EnshrinedAuctioneerEntry<'a>> for DomainEnshrinedAuctioneerEntry {
    fn from(enshrined_auctioneer_entry: EnshrinedAuctioneerEntry<'a>) -> Self {
        let auctioneer_address = DomainAddress::unchecked_from_parts(
            enshrined_auctioneer_entry.auctioneer_address.bytes.into(),
            &enshrined_auctioneer_entry.auctioneer_address.prefix,
        );
        let staker_address = DomainAddress::unchecked_from_parts(
            enshrined_auctioneer_entry.staker_address.bytes.into(),
            &enshrined_auctioneer_entry.staker_address.prefix,
        );
        let asset = match enshrined_auctioneer_entry.asset {
            Denom::TracePrefixed(denom) => DomainDenom::TracePrefixed(denom.into()),
            Denom::IbcPrefixed(denom) => DomainDenom::IbcPrefixed(denom.into()),
        };
        let fee_asset = match enshrined_auctioneer_entry.fee_asset {
            Denom::TracePrefixed(denom) => DomainDenom::TracePrefixed(denom.into()),
            Denom::IbcPrefixed(denom) => DomainDenom::IbcPrefixed(denom.into()),
        };
        DomainEnshrinedAuctioneerEntry {
            auctioneer_address,
            staker_address,
            staked_amount: enshrined_auctioneer_entry.staked_amount,
            fee_asset,
            asset,
        }
    }
}

impl<'a> From<EnshrinedAuctioneerEntry<'a>> for crate::storage::StoredValue<'a> {
    fn from(enshrined_auctioneer_entry: EnshrinedAuctioneerEntry<'a>) -> Self {
        crate::storage::StoredValue::Auctioneer(Value(ValueImpl::EnshrinedAuctioneerEntry(
            enshrined_auctioneer_entry,
        )))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for EnshrinedAuctioneerEntry<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Auctioneer(Value(ValueImpl::EnshrinedAuctioneerEntry(
            enshrined_auctioneer_entry,
        ))) = value
        else {
            bail!(
                "enshrined_auctioneer_entry stored value type mismatch: expected \
                 enshrined_auctioneer_entry, found {value:?}"
            );
        };
        Ok(enshrined_auctioneer_entry)
    }
}
