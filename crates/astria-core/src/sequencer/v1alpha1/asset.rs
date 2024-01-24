use std::{
    fmt,
    fmt::{
        Display,
        Formatter,
    },
};

/// The default sequencer asset base denomination.
pub const DEFAULT_NATIVE_ASSET_DENOM: &str = "nria";

/// Returns the default sequencer asset ID.
#[must_use]
pub fn default_native_asset_id() -> Id {
    Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM).id()
}

/// Represents a denomination of a sequencer asset.
///
/// This can be either an IBC-bridged asset or a native asset.
/// If it's a native asset, the prefix will be empty.
///
/// Note that the full denomination trace of the token is `prefix/base_denom`,
/// in the case that a prefix is present.
/// This is hashed to create the ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Denom {
    id: Id,

    /// The base denomination of the asset; ie. the name of
    /// the smallest unit of the asset.
    base_denom: String,

    /// the IBC denomination prefix.
    prefix: String,
}

impl Denom {
    #[must_use]
    pub fn from_base_denom(base_denom: &str) -> Self {
        let id = Id::from_denom(base_denom);

        Self {
            id,
            base_denom: base_denom.to_string(),
            prefix: String::new(),
        }
    }

    /// Returns the asset ID, which is the hash of the denomination trace.
    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub fn base_denom(&self) -> &str {
        &self.base_denom
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    #[must_use]
    pub fn prefix_is(&self, prefix: &str) -> bool {
        self.prefix == prefix
    }

    #[must_use]
    pub fn denomination_trace(&self) -> String {
        if self.prefix.is_empty() {
            return self.base_denom.clone();
        }

        format!("{}/{}", self.prefix, self.base_denom)
    }
}

impl std::fmt::Display for Denom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.prefix.is_empty() {
            return write!(f, "{}", self.base_denom);
        }

        write!(f, "{}/{}", self.prefix, self.base_denom)
    }
}

/// Creates an `Denom` given a denomination trace.
///
/// Note: if there is no slash in the denomination trace, then
/// it is assumed that the asset is native, and thus the prefix is empty.
impl From<String> for Denom {
    fn from(denom: String) -> Self {
        let Some((prefix, base_denom)) = denom.rsplit_once('/') else {
            return Self {
                id: Id::from_denom(&denom),
                base_denom: denom,
                prefix: String::new(),
            };
        };

        Self {
            id: Id::from_denom(&denom),
            base_denom: base_denom.to_string(),
            prefix: prefix.to_string(),
        }
    }
}

impl From<&str> for Denom {
    fn from(denom: &str) -> Self {
        Self::from(denom.to_string())
    }
}

/// Asset ID, which is the hash of the denomination trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id([u8; 32]);

impl Id {
    #[must_use]
    pub fn from_denom(denom: &str) -> Self {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(denom.as_bytes());
        Self(hash.into())
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
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

impl From<String> for Id {
    fn from(denom: String) -> Self {
        Self::from_denom(&denom)
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Indicates that the protobuf response contained an array field that was not 32 bytes long.
#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectAssetIdLength {
    received: usize,
}
