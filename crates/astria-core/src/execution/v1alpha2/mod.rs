use prost_types::Timestamp;

use crate::{
    generated::execution::v1alpha2 as raw,
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BlockError(BlockErrorKind);

impl BlockError {
    fn field_not_set(field: &'static str) -> Self {
        Self(BlockErrorKind::FieldNotSet(field))
    }

    fn incorrect_block_hash_length(wrong_hash: &[u8]) -> Self {
        Self(BlockErrorKind::IncorrectBlockHashLength(wrong_hash.len()))
    }

    fn incorrect_parent_block_hash_length(wrong_hash: &[u8]) -> Self {
        Self(BlockErrorKind::IncorrectParentBlockHashLength(
            wrong_hash.len(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum BlockErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
    #[error(".hash field contained wrong number of bytes; expected 32, got {0}")]
    IncorrectBlockHashLength(usize),
    #[error(".parent_block_hash field contained wrong number of bytes; expected 32, got {0}")]
    IncorrectParentBlockHashLength(usize),
}

#[derive(Clone, Debug)]
pub struct Block {
    /// The block number
    number: u32,
    /// The hash of the block
    hash: [u8; 32],
    /// The hash from the parent block
    parent_block_hash: [u8; 32],
    /// Timestamp on the block, standardized to google protobuf standard.
    timestamp: Timestamp,
}

impl Block {
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }

    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    #[must_use]
    pub fn parent_block_hash(&self) -> [u8; 32] {
        self.parent_block_hash
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        // prost_types::Timestamp is a (i64, i32) tuple, so this is
        // effectively just a copy
        self.timestamp.clone()
    }
}

impl Protobuf for Block {
    type Error = BlockError;
    type Raw = raw::Block;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::Block {
            number,
            hash,
            parent_block_hash,
            timestamp,
        } = raw;
        let hash = hash
            .as_slice()
            .try_into()
            .map_err(|_| Self::Error::incorrect_block_hash_length(hash))?;
        let parent_block_hash = parent_block_hash
            .as_slice()
            .try_into()
            .map_err(|_| Self::Error::incorrect_parent_block_hash_length(parent_block_hash))?;

        // Clone'ing timestamp is effectively a copy because timestamp is just a (i32, i64) tuple
        let timestamp = timestamp
            .clone()
            .ok_or(Self::Error::field_not_set(".timestamp"))?;

        Ok(Self {
            number: *number,
            hash,
            parent_block_hash,
            timestamp,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            number,
            hash,
            parent_block_hash,
            timestamp,
        } = self;
        Self::Raw {
            number: *number,
            hash: hash.to_vec(),
            parent_block_hash: parent_block_hash.to_vec(),
            // Clone'ing timestamp is effectively a copy because timestamp is just a (i32, i64)
            // tuple
            timestamp: Some(timestamp.clone()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CommitmentStateError(CommitmentStateErrorKind);

impl CommitmentStateError {
    fn field_not_set(field: &'static str) -> Self {
        Self(CommitmentStateErrorKind::FieldNotSet(field))
    }

    fn firm(source: BlockError) -> Self {
        Self(CommitmentStateErrorKind::Firm(source))
    }

    fn soft(source: BlockError) -> Self {
        Self(CommitmentStateErrorKind::Soft(source))
    }
}

#[derive(Debug, thiserror::Error)]
enum CommitmentStateErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
    #[error(".firm field did not contain a valid block")]
    Firm(#[source] BlockError),
    #[error(".soft field did not contain a valid block")]
    Soft(#[source] BlockError),
}

/// The CommitmentState holds the block at each stage of sequencer commitment
/// level
///
/// A Valid CommitmentState:
/// - Block numbers are such that soft >= firm.
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the head of the canonical chain the firm block must belong to.
#[derive(Clone, Debug)]
pub struct CommitmentState {
    /// Soft commitment is the rollup block matching latest sequencer block.
    pub soft: Block,
    /// Firm commitment is achieved when data has been seen in DA.
    pub firm: Block,
}

impl CommitmentState {
    #[must_use]
    pub fn firm(&self) -> &Block {
        &self.firm
    }

    #[must_use]
    pub fn soft(&self) -> &Block {
        &self.soft
    }
}

impl Protobuf for CommitmentState {
    type Error = CommitmentStateError;
    type Raw = raw::CommitmentState;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            soft,
            firm,
        } = raw;
        let soft = 'soft: {
            let Some(soft) = soft else {
                break 'soft Err(Self::Error::field_not_set(".soft"));
            };
            Block::try_from_raw_ref(soft).map_err(Self::Error::soft)
        }?;
        let firm = 'firm: {
            let Some(firm) = firm else {
                break 'firm Err(Self::Error::field_not_set(".firm"));
            };
            Block::try_from_raw_ref(firm).map_err(Self::Error::firm)
        }?;
        Ok(Self {
            soft,
            firm,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            soft,
            firm,
        } = self;
        let soft = soft.to_raw();
        let firm = firm.to_raw();
        Self::Raw {
            soft: Some(soft),
            firm: Some(firm),
        }
    }
}
