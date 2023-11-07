use std::{
    error::Error,
    fmt::Display,
};

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
            base_denom: base_denom.to_owned(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &Id {
        &self.id
    }

    #[must_use]
    pub fn base_denom(&self) -> &str {
        &self.base_denom
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

/// Indicates that the protobuf response contained an array field that was not 32 bytes long.
#[derive(Debug)]
pub struct IncorrectAssetIdLength {
    received: usize,
}

impl Display for IncorrectAssetIdLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 32 bytes, got {}", self.received)
    }
}

impl Error for IncorrectAssetIdLength {}
