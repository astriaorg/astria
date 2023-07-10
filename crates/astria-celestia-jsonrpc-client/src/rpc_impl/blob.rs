//! The Celestia JSON RPC blob API.
//!
//! Many of the constants are taken from these two sources:
//!
//! + [celestia-app:965eaf global_consts.go](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/appconsts/global_consts.go)
//! + [celestia-app:965eaf namespace/consts.go](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/namespace/consts.go)
use jsonrpsee::proc_macros::rpc;
use serde::{
    Deserialize,
    Serialize,
};

use crate::serde::Base64Standard;

/// The full theoretical length of the celestia namespace ID in bytes.
///
/// In practice the actual length available (at least for version 0 of the namespace
/// API) is [`NAMESPACE_ID_ACTUAL_LEN`] (currently 10 bytes).
///
/// From [celestia-app:965eaf global_consts.go#L22](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/appconsts/global_consts.go#L22)
const NAMESPACE_ID_LEN: usize = 28;

/// The length of the celestia namespace version in bytes.
///
/// From [celestia-app:965eaf global_consts.go#L16](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/appconsts/global_consts.go#L16)
pub(crate) const NAMESPACE_VERSION_LEN: usize = 1;

/// The total length of the celestia namespace byte slice as ingested by their API.
///
/// Currently defined as the length of the namespace ID and the length of the namespace version.
/// From [celestia-app:965eaf global_consts.go#L25](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/appconsts/global_consts.go#L25)
const NAMESPACE_LEN: usize = NAMESPACE_ID_LEN + NAMESPACE_VERSION_LEN;

/// The number of zeros that the namespace ID is prefixed withe for version 0 of the namespace API.
///
/// From [celestia-app:965eaf namespace/consts.go#L28](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/namespace/consts.go#L28)
pub(crate) const NAMESPACE_VERSION_ZERO_PREFIX_LEN: usize = 18;

/// The actual number of bytes that is available to the user to construct a namespace.
///
/// From [celestia-app:965eaf namespace/consts.go#L32](https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/pkg/namespace/consts.go#L32)
pub const NAMESPACE_ID_AVAILABLE_LEN: usize = NAMESPACE_ID_LEN - NAMESPACE_VERSION_ZERO_PREFIX_LEN;

/// The commitment of a blob.
///
/// At this time it is not clear how this is constructed, so should probably be left empty.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Commitment(#[serde(with = "Base64Standard")] Vec<u8>);

impl Commitment {
    /// Construct an empty commitment.
    #[must_use]
    pub fn empty() -> Self {
        Commitment::default()
    }
}

/// The celestia namespace.
///
/// Currently defined as a version byte + a 28 bytes.
#[derive(Debug, Default, Serialize)]
pub struct Namespace(#[serde(serialize_with = "Base64Standard::serialize")] [u8; NAMESPACE_LEN]);

impl Namespace {
    /// Constructs a new version 0 namespace with the given a namespace ID.
    #[must_use]
    pub fn new_v0(id: [u8; NAMESPACE_ID_AVAILABLE_LEN]) -> Self {
        let mut namespace = [0u8; NAMESPACE_LEN];
        namespace[NAMESPACE_VERSION_LEN + NAMESPACE_VERSION_ZERO_PREFIX_LEN..].copy_from_slice(&id);
        Self(namespace)
    }
}

impl std::ops::Deref for Namespace {
    type Target = [u8; NAMESPACE_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let buf: Vec<u8> = Base64Standard::deserialize(deserializer)?;
        let namespace = match buf.try_into() {
            Err(_) => {
                return Err(D::Error::custom(
                    "received a namespace of length other than 29 bytes",
                ));
            }
            Ok(namespace) => namespace,
        };
        Ok(Self(namespace))
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Blob {
    pub namespace: Namespace,
    #[serde(with = "Base64Standard")]
    pub data: Vec<u8>,
    /// Currently only a share version of 0 seems to be supported.
    ///
    /// From: [https://github.com/celestiaorg/celestia-app/blob/965eafb4357376aec31f84f3628f7703c5587f9a/x/blob/types/payforblob.go#L114C1]
    pub share_version: u32,
    pub commitment: Commitment,
}

#[rpc(client)]
pub trait Blob {
    #[method(name = "blob.Get")]
    async fn get(
        &self,
        height: u64,
        namespace: Namespace,
        commitment: Commitment,
    ) -> Result<serde_json::Value, Error>;

    #[method(name = "blob.GetAll")]
    async fn get_all(
        &self,
        height: u64,
        namespace: &[Namespace],
    ) -> Result<Box<serde_json::value::RawValue>, Error>;

    #[method(name = "blob.Submit")]
    async fn submit(&self, blobs: &[Blob]) -> Result<Box<serde_json::value::RawValue>, Error>;
}
