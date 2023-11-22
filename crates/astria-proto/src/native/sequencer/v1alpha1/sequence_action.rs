use crate::generated::sequencer::v1alpha1 as raw;

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub chain_id: Vec<u8>,
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
            chain_id,
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
            chain_id: chain_id.clone(),
            data: data.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SequenceAction`].
    #[must_use]
    pub fn from_raw(proto: raw::SequenceAction) -> Self {
        let raw::SequenceAction {
            chain_id,
            data,
        } = proto;
        Self {
            chain_id,
            data,
        }
    }
}
