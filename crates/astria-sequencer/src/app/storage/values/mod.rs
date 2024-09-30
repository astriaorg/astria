mod block_height;
mod block_timestamp;
mod chain_id;
mod revision_number;
mod storage_version;

use std::fmt::{
    self,
    Display,
    Formatter,
};

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

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ValueImpl::ChainId(chain_id) => write!(f, "chain id {chain_id}"),
            ValueImpl::RevisionNumber(revision_number) => {
                write!(f, "revision number {revision_number}")
            }
            ValueImpl::BlockHeight(block_height) => write!(f, "block height {block_height}"),
            ValueImpl::BlockTimestamp(block_timestamp) => {
                write!(f, "block timestamp {block_timestamp}")
            }
            ValueImpl::StorageVersion(storage_version) => {
                write!(f, "storage version {storage_version}")
            }
        }
    }
}
