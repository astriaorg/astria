mod block_height;
mod block_timestamp;
mod chain_id;
mod revision_number;
mod storage_version;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::app) use self::{
    block_height::BlockHeight,
    block_timestamp::BlockTimestamp,
    chain_id::ChainId,
    revision_number::RevisionNumber,
    storage_version::StorageVersion,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    ChainId(ChainId<'a>),
    RevisionNumber(RevisionNumber),
    BlockHeight(BlockHeight),
    BlockTimestamp(BlockTimestamp),
    StorageVersion(StorageVersion),
}
