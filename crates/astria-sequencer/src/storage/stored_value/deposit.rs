use std::borrow::Cow;

use astria_core::{
    primitive::v1::{
        asset::Denom as DomainDenom,
        Address as DomainAddress,
        TransactionId as DomainTransactionId,
    },
    sequencerblock::v1alpha1::block::Deposit as DomainDeposit,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    AddressBytes,
    IbcPrefixedDenom,
    RollupId,
    StoredValue,
    TracePrefixedDenom,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Address<'a> {
    bytes: AddressBytes<'a>,
    prefix: Cow<'a, str>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum Denom<'a> {
    TracePrefixed(TracePrefixedDenom<'a>),
    IbcPrefixed(IbcPrefixedDenom<'a>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct TransactionId<'a>(Cow<'a, [u8; 32]>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Deposit<'a> {
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
            bytes: deposit.bridge_address.bytes().into(),
            prefix: Cow::Borrowed(deposit.bridge_address.prefix()),
        };
        let asset = match &deposit.asset {
            DomainDenom::TracePrefixed(denom) => Denom::TracePrefixed(denom.into()),
            DomainDenom::IbcPrefixed(denom) => Denom::IbcPrefixed(denom.into()),
        };
        let source_transaction_id =
            TransactionId(Cow::Borrowed(deposit.source_transaction_id.get()));
        Deposit {
            bridge_address,
            rollup_id: RollupId::from(&deposit.rollup_id),
            amount: deposit.amount,
            asset,
            destination_chain_address: Cow::Borrowed(&deposit.destination_chain_address),
            source_transaction_id,
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
        let source_transaction_id =
            DomainTransactionId::new(deposit.source_transaction_id.0.into_owned());
        DomainDeposit {
            bridge_address,
            rollup_id: deposit.rollup_id.into(),
            amount: deposit.amount,
            asset,
            destination_chain_address: deposit.destination_chain_address.into(),
            source_transaction_id,
            source_action_index: deposit.source_action_index,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Deposits<'a>(Vec<Deposit<'a>>);

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

impl<'a> TryFrom<StoredValue<'a>> for Deposits<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::Deposits(deposits) = value else {
            return Err(super::type_mismatch("deposits", &value));
        };
        Ok(deposits)
    }
}
