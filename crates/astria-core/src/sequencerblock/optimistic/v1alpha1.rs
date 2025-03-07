use bytes::Bytes;

use crate::{
    generated::astria::sequencerblock::optimistic::v1alpha1 as raw,
    sequencerblock::v1::block,
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequencerBlockCommitError(SequencerBlockCommitErrorKind);

impl SequencerBlockCommitError {
    fn block_hash(source: block::HashFromSliceError) -> Self {
        Self(SequencerBlockCommitErrorKind::BlockHash {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum SequencerBlockCommitErrorKind {
    #[error("failed to read .block_hash field as sequencer block hash")]
    BlockHash { source: block::HashFromSliceError },
}

#[derive(Clone, Debug)]
pub struct SequencerBlockCommit {
    height: u64,
    block_hash: block::Hash,
}

impl SequencerBlockCommit {
    #[must_use]
    pub fn new(height: u64, block_hash: block::Hash) -> Self {
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
    pub fn block_hash(&self) -> &block::Hash {
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

        let block_hash =
            block::Hash::try_from(&**block_hash).map_err(SequencerBlockCommitError::block_hash)?;

        Ok(SequencerBlockCommit {
            height: *height,
            block_hash,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        raw::SequencerBlockCommit {
            height: self.height(),
            block_hash: Bytes::copy_from_slice(self.block_hash.as_bytes()),
        }
    }
}
