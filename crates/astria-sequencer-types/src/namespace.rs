use std::{
    fmt,
    ops::Deref,
};

use astria_celestia_jsonrpc_client::blob::NAMESPACE_ID_AVAILABLE_LEN;
use serde::{
    de::{self,},
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

    /// returns an 10-byte namespace given a byte slice by hashing
    /// the bytes with sha256 and returning the first 10 bytes.
    #[must_use]
    pub fn with_hashed_bytes(bytes: &[u8]) -> Namespace {
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
        hex::serialize(self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Namespace, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = hex::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(Namespace::new(bytes))
    }
}
