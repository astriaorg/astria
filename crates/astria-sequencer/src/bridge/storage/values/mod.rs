mod address_bytes;
mod block_height;
mod deposits;
mod ibc_prefixed_denom;
mod rollup_id;
mod transaction_id;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::bridge) use self::{
    address_bytes::AddressBytes,
    block_height::BlockHeight,
    deposits::Deposits,
    ibc_prefixed_denom::IbcPrefixedDenom,
    rollup_id::RollupId,
    transaction_id::TransactionId,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    RollupId(RollupId<'a>),
    IbcPrefixedDenom(IbcPrefixedDenom<'a>),
    AddressBytes(AddressBytes<'a>),
    BlockHeight(BlockHeight),
    Deposits(Deposits<'a>),
    TransactionId(TransactionId<'a>),
}
