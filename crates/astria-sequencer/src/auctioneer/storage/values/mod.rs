use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

mod address_bytes;
mod auctioneer_entry;
mod block_height;
mod ibc_prefixed_denom;

pub(in crate::auctioneer) use self::{
    address_bytes::AddressBytes,
    auctioneer_entry::EnshrinedAuctioneerEntry,
    block_height::BlockHeight,
    ibc_prefixed_denom::IbcPrefixedDenom,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    AddressBytes(AddressBytes<'a>),
    BlockHeight(BlockHeight),
    IbcPrefixedDenom(IbcPrefixedDenom<'a>),
    EnshrinedAuctioneerEntry(EnshrinedAuctioneerEntry<'a>),
}
