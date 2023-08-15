use std::{
    fmt,
    ops::Deref,
};

use astria_celestia_jsonrpc_client::blob::NAMESPACE_ID_AVAILABLE_LEN;
use eyre::{
    ensure,
    WrapErr as _,
};
use serde::{
    de::{
        self,
        Visitor,
    },
    Deserialize,
    Deserializer,
    Serialize,
};
use sha2::{
    Digest,
    Sha256,
};

/// The default namespace blocks are written to.
/// A block in this namespace contains "pointers" to the rollup txs contained
/// in that block; ie. a list of tuples of (DA block height, namespace).
pub static DEFAULT_NAMESPACE: Namespace = Namespace(*b"astriasequ");

/// Namespace represents a Celestia namespace.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Namespace([u8; NAMESPACE_ID_AVAILABLE_LEN]);

impl Deref for Namespace {
    type Target = [u8; NAMESPACE_ID_AVAILABLE_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Namespace {
    #[must_use]
    pub fn new(inner: [u8; NAMESPACE_ID_AVAILABLE_LEN]) -> Self {
        Self(inner)
    }

    /// Creates a namespace from a 10-byte hex-encoded string.
    ///
    /// # Errors
    ///
    /// - if the string cannot be deocded as hex
    /// - if the string does not contain 10 bytes
    pub fn from_string(s: &str) -> eyre::Result<Self> {
        let bytes = hex::decode(s).wrap_err("failed reading string as hex encoded bytes")?;
        ensure!(
            bytes.len() == NAMESPACE_ID_AVAILABLE_LEN,
            "string encoded wrong number of bytes",
        );
        let mut namespace = [0u8; NAMESPACE_ID_AVAILABLE_LEN];
        namespace.copy_from_slice(&bytes);
        Ok(Namespace(namespace))
    }

    /// returns an 10-byte namespace given a byte slice by hashing
    /// the bytes with sha256 and returning the first 10 bytes.
    #[must_use]
    pub fn new_from_bytes(bytes: &[u8]) -> Namespace {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        Namespace(
            result[0..NAMESPACE_ID_AVAILABLE_LEN]
                .to_owned()
                .try_into()
                .expect("cannot fail as hash is always 32 bytes"),
        )
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // FIXME: `hex::encode` does an extra allocation which could be removed
        f.write_str(&hex::encode(self.0))
    }
}

impl Serialize for Namespace {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Namespace, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(NamespaceVisitor)
    }
}

struct NamespaceVisitor;

impl NamespaceVisitor {
    fn decode_string<E>(value: &str) -> Result<Namespace, E>
    where
        E: de::Error,
    {
        Namespace::from_string(value).map_err(|e| de::Error::custom(format!("{e:?}")))
    }
}

impl<'de> Visitor<'de> for NamespaceVisitor {
    type Value = Namespace;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing 8 hex-encoded bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::decode_string(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::decode_string(&value)
    }
}
