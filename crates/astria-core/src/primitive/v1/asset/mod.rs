use std::fmt::{
    self,
    Display,
    Formatter,
};

pub mod denom;
pub use denom::{
    Denom,
    ParseDenomError,
};

/// The default sequencer asset base denomination.
pub const DEFAULT_NATIVE_ASSET_DENOM: &str = "nria";

/// Returns the default sequencer asset [`Denom`].
// allow: parsing single-segment assets is unit tested
#[allow(clippy::missing_panics_doc)]
// allow: used in many places already
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn default_native_asset() -> Denom {
    DEFAULT_NATIVE_ASSET_DENOM
        .parse()
        .expect("parsing a single segment string must work")
}

/// Asset ID, which is the hash of the denomination trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Id(
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serde::base64_serialize")
    )]
    [u8; 32],
);

impl Id {
    #[must_use]
    pub fn new(arr: [u8; 32]) -> Self {
        Self(arr)
    }

    /// Constructs an ID by hashing `s` without checking if `s` is a valid denom.
    #[must_use]
    pub fn from_str_unchecked(s: &str) -> Self {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(s.as_bytes());
        Self(hash.into())
    }

    #[must_use]
    pub fn get(self) -> [u8; 32] {
        self.0
    }

    /// Returns an ID given a 32-byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not 32 bytes long.
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, IncorrectAssetIdLength> {
        if slice.len() != 32 {
            return Err(IncorrectAssetIdLength {
                received: slice.len(),
            });
        }

        let mut id = [0u8; 32];
        id.copy_from_slice(slice);
        Ok(Self(id))
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use base64::{
            display::Base64Display,
            prelude::BASE64_STANDARD,
        };
        Base64Display::new(self.as_ref(), &BASE64_STANDARD).fmt(f)
    }
}

/// Indicates that the protobuf response contained an array field that was not 32 bytes long.
#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectAssetIdLength {
    received: usize,
}
