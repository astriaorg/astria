//! Utilities for serializing and deserializing bytes
use std::collections::HashMap;

use base64_serde::base64_serde_type;
use serde::{
    ser::SerializeMap,
    Serialize,
};

use crate::types::{
    IndexedTransaction,
    Namespace,
};

base64_serde_type!(pub(crate) Base64Standard, base64::engine::general_purpose::STANDARD);

pub(crate) struct NamespaceToTxCount<'a>(
    pub(crate) &'a HashMap<Namespace, Vec<IndexedTransaction>>,
);

impl<'a> Serialize for NamespaceToTxCount<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (ns, txs) in self.0 {
            map.serialize_entry(&ns, &txs.len())?;
        }
        map.end()
    }
}

impl<'a> std::fmt::Display for NamespaceToTxCount<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Note that this technically throws away the error, but that's
        // ok because the serialization to serde can only fail if the key
        // in the map is not valid utf8. However, since we control they
        // key and guarantee that it is valid utf8 this is ok.
        f.write_str(&serde_json::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
