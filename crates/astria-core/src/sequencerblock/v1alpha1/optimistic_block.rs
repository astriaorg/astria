use bytes::Bytes;

use crate::{
    generated::sequencerblock::v1alpha1 as raw,
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequencerBlockCommitError(SequencerBlockCommitErrorKind);

impl SequencerBlockCommitError {
    fn invalid_block_hash(len: usize) -> Self {
        Self(SequencerBlockCommitErrorKind::InvalidBlockHash(len))
    }
}

#[derive(Debug, thiserror::Error)]
enum SequencerBlockCommitErrorKind {
    #[error("invalid block hash length: {0}")]
    InvalidBlockHash(usize),
}

#[derive(Clone, Debug)]
pub struct SequencerBlockCommit {
    height: u64,
    block_hash: [u8; 32],
}

impl SequencerBlockCommit {
    #[must_use]
    pub fn new(height: u64, block_hash: [u8; 32]) -> Self {
        Self {
            height,
            block_hash,
        }
    }

    #[must_use]
    pub fn height(&self) -> u64 {
        self.height
    }

    #[must_use]
    pub fn block_hash(&self) -> &[u8; 32] {
        &self.block_hash
    }
}

impl From<SequencerBlockCommit> for raw::SequencerBlockCommit {
    fn from(value: SequencerBlockCommit) -> Self {
        value.to_raw()
    }
}

impl Protobuf for SequencerBlockCommit {
    type Error = SequencerBlockCommitError;
    type Raw = raw::SequencerBlockCommit;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            height,
            block_hash,
        } = raw;

        let block_hash = block_hash
            .as_ref()
            .try_into()
            .map_err(|_| SequencerBlockCommitError::invalid_block_hash(block_hash.len()))?;

        Ok(SequencerBlockCommit {
            height: *height,
            block_hash,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            height,
            block_hash,
        } = self;

        raw::SequencerBlockCommit {
            height: *height,
            block_hash: Bytes::copy_from_slice(block_hash),
        }
    }
}
