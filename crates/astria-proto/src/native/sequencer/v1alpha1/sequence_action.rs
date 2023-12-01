use std::{
    error::Error,
    fmt::Display,
};

use super::{
    IncorrectRollupIdLength,
    RollupId,
};
use crate::generated::sequencer::v1alpha1 as raw;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct SequenceActionError {
    kind: SequenceActionErrorKind,
}

impl SequenceActionError {
    fn rollup_id(inner: IncorrectRollupIdLength) -> Self {
        Self {
            kind: SequenceActionErrorKind::RollupId(inner),
        }
    }
}

impl Display for SequenceActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SequenceActionErrorKind::RollupId(_) => {
                f.pad("`rollup_id` field did not contain a valid rollup ID")
            }
        }
    }
}

impl Error for SequenceActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SequenceActionErrorKind::RollupId(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SequenceActionErrorKind {
    RollupId(IncorrectRollupIdLength),
}

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub rollup_id: RollupId,
    pub data: Vec<u8>,
}

impl SequenceAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data,
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data: data.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SequenceAction`].
    ///
    /// # Errors
    /// Returns an error if the `proto.rollup_id` field was not 32 bytes.
    pub fn try_from_raw(proto: raw::SequenceAction) -> Result<Self, SequenceActionError> {
        let raw::SequenceAction {
            rollup_id,
            data,
        } = proto;
        let rollup_id =
            RollupId::try_from_slice(&rollup_id).map_err(SequenceActionError::rollup_id)?;
        Ok(Self {
            rollup_id,
            data,
        })
    }
}
