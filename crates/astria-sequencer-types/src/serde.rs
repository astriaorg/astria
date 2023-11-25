//! Utilities for serializing and deserializing bytes
use std::collections::BTreeMap;

use base64_serde::base64_serde_type;
use proto::native::sequencer::v1alpha1::ChainId;
use serde::{
    ser::SerializeMap,
    Serialize,
};

base64_serde_type!(pub Base64Standard, base64::engine::general_purpose::STANDARD);

pub struct ChainIdToTxCount<'a>(pub(crate) &'a BTreeMap<ChainId, Vec<Vec<u8>>>);

impl<'a> ChainIdToTxCount<'a> {
    #[must_use]
    pub fn new(rollup_data: &BTreeMap<ChainId, Vec<Vec<u8>>>) -> ChainIdToTxCount {
        ChainIdToTxCount(rollup_data)
    }
}

impl<'a> Serialize for ChainIdToTxCount<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (ns, data) in self.0 {
            map.serialize_entry(&ns, &data.len())?;
        }
        map.end()
    }
}

impl<'a> std::fmt::Display for ChainIdToTxCount<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This cannot fail because we are only serializing into a string (unless the system is
        // OOM).
        f.write_str(&serde_json::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
