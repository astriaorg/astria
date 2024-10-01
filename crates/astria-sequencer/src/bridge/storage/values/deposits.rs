use std::borrow::Cow;

use astria_core::{
    primitive::v1::{
        asset::{
            Denom as DomainDenom,
            TracePrefixed as DomainTracePrefixed,
        },
        Address as DomainAddress,
    },
    sequencerblock::v1alpha1::block::Deposit as DomainDeposit,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    AddressBytes,
    IbcPrefixedDenom,
    RollupId,
    TransactionId,
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Address<'a> {
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
enum Denom<'a> {
    TracePrefixed(TracePrefixedDenom<'a>),
    IbcPrefixed(IbcPrefixedDenom<'a>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Deposit<'a> {
    bridge_address: Address<'a>,
    rollup_id: RollupId<'a>,
    amount: u128,
    asset: Denom<'a>,
    destination_chain_address: Cow<'a, str>,
    source_transaction_id: TransactionId<'a>,
    source_action_index: u64,
}

impl<'a> From<&'a DomainDeposit> for Deposit<'a> {
    fn from(deposit: &'a DomainDeposit) -> Self {
        let bridge_address = Address {
            bytes: deposit.bridge_address.as_bytes().into(),
            prefix: Cow::Borrowed(deposit.bridge_address.prefix()),
        };
        let asset = match &deposit.asset {
            DomainDenom::TracePrefixed(denom) => Denom::TracePrefixed(denom.into()),
            DomainDenom::IbcPrefixed(denom) => Denom::IbcPrefixed(denom.into()),
        };
        Deposit {
            bridge_address,
            rollup_id: RollupId::from(&deposit.rollup_id),
            amount: deposit.amount,
            asset,
            destination_chain_address: Cow::Borrowed(&deposit.destination_chain_address),
            source_transaction_id: TransactionId::from(&deposit.source_transaction_id),
            source_action_index: deposit.source_action_index,
        }
    }
}

impl<'a> From<Deposit<'a>> for DomainDeposit {
    fn from(deposit: Deposit<'a>) -> Self {
        let bridge_address = DomainAddress::unchecked_from_parts(
            deposit.bridge_address.bytes.into(),
            &deposit.bridge_address.prefix,
        );
        let asset = match deposit.asset {
            Denom::TracePrefixed(denom) => DomainDenom::TracePrefixed(denom.into()),
            Denom::IbcPrefixed(denom) => DomainDenom::IbcPrefixed(denom.into()),
        };
        DomainDeposit {
            bridge_address,
            rollup_id: deposit.rollup_id.into(),
            amount: deposit.amount,
            asset,
            destination_chain_address: deposit.destination_chain_address.into(),
            source_transaction_id: deposit.source_transaction_id.into(),
            source_action_index: deposit.source_action_index,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::bridge) struct Deposits<'a>(Vec<Deposit<'a>>);

impl<'a, T: Iterator<Item = &'a DomainDeposit>> From<T> for Deposits<'a> {
    fn from(deposit_iter: T) -> Self {
        Deposits(deposit_iter.map(Deposit::from).collect())
    }
}

impl<'a> From<Deposits<'a>> for Vec<DomainDeposit> {
    fn from(deposits: Deposits<'a>) -> Self {
        deposits.0.into_iter().map(DomainDeposit::from).collect()
    }
}

impl<'a> From<Deposits<'a>> for crate::storage::StoredValue<'a> {
    fn from(deposits: Deposits<'a>) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::Deposits(deposits)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Deposits<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::Deposits(deposits))) = value
        else {
            bail!("bridge stored value type mismatch: expected deposits, found {value:?}");
        };
        Ok(deposits)
    }
}
