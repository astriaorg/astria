mod address_bytes;
mod block_height;
mod deposits;
mod fee;
mod ibc_prefixed_denom;
mod rollup_id;
mod transaction_id;

use std::fmt::{
    self,
    Display,
    Formatter,
};

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(crate) use self::{
    address_bytes::AddressBytes,
    block_height::BlockHeight,
    deposits::Deposits,
    fee::Fee,
    ibc_prefixed_denom::IbcPrefixedDenom,
    rollup_id::RollupId,
    transaction_id::TransactionId,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum Value<'a> {
    RollupId(RollupId<'a>),
    IbcPrefixedDenom(IbcPrefixedDenom<'a>),
    AddressBytes(AddressBytes<'a>),
    BlockHeight(BlockHeight),
    Deposits(Deposits<'a>),
    Fee(Fee),
    TransactionId(TransactionId<'a>),
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::RollupId(rollup_id) => write!(f, "rollup id {rollup_id}"),
            Value::IbcPrefixedDenom(denom) => write!(f, "denom {denom}"),
            Value::AddressBytes(address_bytes) => write!(f, "address bytes {address_bytes}"),
            Value::BlockHeight(block_height) => write!(f, "block height {block_height}"),
            Value::Deposits(_deposits) => write!(f, "deposits"),
            Value::Fee(fee) => write!(f, "fee {fee}"),
            Value::TransactionId(tx_id) => write!(f, "transaction id {tx_id}"),
        }
    }
}
