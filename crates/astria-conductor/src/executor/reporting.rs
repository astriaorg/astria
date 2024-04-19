//! Various newtype-wrappers to emit serde-serialized tracing event fields.
use serde::ser::{
    Serialize,
    SerializeSeq,
};

pub(super) struct ReportBlocks<'a, T>(pub(super) &'a [T]);

impl<'a> Serialize for ReportBlocks<'a, astria_core::execution::v1alpha2::Block> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for elem in self.0 {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}
