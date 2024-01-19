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
#[derive(Debug, Clone)]
pub struct Denom {
    id: Id,

    /// The base denomination of the asset; ie. the name of
    /// the smallest unit of the asset.
    base_denom: String,
}

impl Denom {
    #[must_use]
    pub fn from_base_denom(base_denom: &str) -> Self {
        let id = Id::from_denom(base_denom);

        Self {
            id,
            base_denom: base_denom.to_string(),
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
}

impl From<String> for Denom {
    fn from(base_denom: String) -> Self {
        Self::from_base_denom(&base_denom)
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

/// Represents an IBC asset.
///
/// Note that the full denomination trace of the token is `prefix/base_denom`.
/// This is hashed to create the ID.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct IbcAsset {
    id: Id,

    /// The base denomination of the asset; ie. the name of
    /// the smallest unit of the asset.
    base_denom: String,

    /// the IBC denomination prefix.
    prefix: String,
}

impl IbcAsset {
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
        format!("{}/{}", self.prefix, self.base_denom)
    }
}

impl std::fmt::Display for IbcAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.prefix, self.base_denom)
    }
}

/// Creates an `IbcAsset` given a denomination trace.
///
/// # Errors
///
/// - if the denomination string is invalid, ie. does not contain any slashes.
impl std::str::FromStr for IbcAsset {
    type Err = IbcAssetError;

    fn from_str(denom: &str) -> Result<Self, Self::Err> {
        let Some((prefix, base_denom)) = denom.rsplit_once('/') else {
            return Err(IbcAssetError::invalid_denomination());
        };
        let id = Id::from_denom(denom);

        Ok(Self {
            id,
            base_denom: base_denom.to_string(),
            prefix: prefix.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IbcAssetError(IbcAssetErrorKind);

impl IbcAssetError {
    fn invalid_denomination() -> Self {
        Self(IbcAssetErrorKind::InvalidDenomination)
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcAssetErrorKind {
    #[error("denomination must contain at least one slash")]
    InvalidDenomination,
}
