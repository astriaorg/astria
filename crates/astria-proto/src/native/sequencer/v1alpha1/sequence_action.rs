use std::{
    error::Error,
    fmt::Display,
};

use super::{
    ChainId,
    IncorrectChainIdLength,
};
use crate::generated::sequencer::v1alpha1 as raw;

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub chain_id: ChainId,
    pub data: Vec<u8>,
}

impl SequenceAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SequenceAction {
        let Self {
            chain_id,
            data,
        } = self;
        raw::SequenceAction {
            chain_id: chain_id.to_vec(),
            data,
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            chain_id,
            data,
        } = self;
        raw::SequenceAction {
            chain_id: chain_id.to_vec(),
            data: data.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SequenceAction`].
    ///
    /// # Errors
    /// Returns an error if the `proto.chain_id` field was not 32 bytes.
    pub fn try_from_raw(proto: raw::SequenceAction) -> Result<Self, SequenceActionError> {
        let raw::SequenceAction {
            chain_id,
            data,
        } = proto;
        let chain_id = ChainId::try_from_slice(&chain_id).map_err(SequenceActionError::chain_id)?;
        Ok(Self {
            chain_id,
            data,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct SequenceActionError {
    kind: SequenceActionErrorKind,
}

impl SequenceActionError {
    fn chain_id(inner: IncorrectChainIdLength) -> Self {
        Self {
            kind: SequenceActionErrorKind::ChainId(inner),
        }
    }
}
impl Display for SequenceActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SequenceActionErrorKind::ChainId(_) => {
                f.pad("`chain_id` field did not contain a valid chain ID")
            }
        }
    }
}

impl Error for SequenceActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SequenceActionErrorKind::ChainId(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SequenceActionErrorKind {
    ChainId(IncorrectChainIdLength),
}
