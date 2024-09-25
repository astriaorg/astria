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

pub(crate) use self::{
    block_height::BlockHeight,
    block_timestamp::BlockTimestamp,
    chain_id::ChainId,
    revision_number::RevisionNumber,
    storage_version::StorageVersion,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum Value<'a> {
    ChainId(ChainId<'a>),
    RevisionNumber(RevisionNumber),
    BlockHeight(BlockHeight),
    BlockTimestamp(BlockTimestamp),
    StorageVersion(StorageVersion),
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::ChainId(chain_id) => write!(f, "chain id {chain_id}"),
            Value::RevisionNumber(revision_number) => {
                write!(f, "revision number {revision_number}")
            }
            Value::BlockHeight(block_height) => write!(f, "block height {block_height}"),
            Value::BlockTimestamp(block_timestamp) => {
                write!(f, "block timestamp {block_timestamp}")
            }
            Value::StorageVersion(storage_version) => {
                write!(f, "storage version {storage_version}")
            }
        }
    }
}
