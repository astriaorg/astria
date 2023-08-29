//! Utilities for serializing and deserializing bytes
use std::collections::HashMap;

use base64_serde::base64_serde_type;
use serde::{
    ser::SerializeMap,
    Serialize,
};

use crate::{
    Namespace,
    RollupData,
};

base64_serde_type!(pub Base64Standard, base64::engine::general_purpose::STANDARD);

pub mod chain_id {
    //! Helper functions to serialize and deserialize [`ChainId`].
    //!
    //! To be used in `#[serde(with = "crate::serde::chain_id")]` attributes
    //! when deriving `Deserialize` and `Serialize` on types containing a `ChainId`.
    use proto::native::sequencer::v1alpha1::ChainId;
    use serde::{
        Deserializer,
        Serializer,
    };
    /// Utility to deserialize bytes into a [`ChainId`].
    ///
    /// # Errors
    ///
    /// Returns the same error as [`hex::serde::deserialize`] if the input was not
    /// hex formatted or did not encode 32 bytes.
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<ChainId, D::Error> {
        use proto::native::sequencer::v1alpha1::CHAIN_ID_LEN;
        let inner: [u8; CHAIN_ID_LEN] = hex::serde::deserialize(de)?;
        Ok(ChainId(inner))
    }

    /// Utility to serialize [`ChainId`] to a hex encoded byte string.
    ///
    /// # Errors
    ///
    /// Returns the same error as [`hex::serde::serialize`].
    pub fn serialize<S: Serializer>(val: &ChainId, se: S) -> Result<S::Ok, S::Error> {
        hex::serde::serialize(val, se)
    }
}

pub struct NamespaceToTxCount<'a>(pub(crate) &'a HashMap<Namespace, RollupData>);

impl<'a> NamespaceToTxCount<'a> {
    #[must_use]
    pub fn new(rollup_data: &HashMap<Namespace, RollupData>) -> NamespaceToTxCount {
        NamespaceToTxCount(rollup_data)
    }
}

impl<'a> Serialize for NamespaceToTxCount<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (ns, data) in self.0 {
            map.serialize_entry(&ns, &data.transactions.len())?;
        }
        map.end()
    }
}

impl<'a> std::fmt::Display for NamespaceToTxCount<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This cannot fail because we are only serializing into a string (unless the system is
        // OOM).
        f.write_str(&serde_json::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
