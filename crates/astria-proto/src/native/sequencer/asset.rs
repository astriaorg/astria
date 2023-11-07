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
        let id = Id::from(base_denom);

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

impl From<&str> for Id {
    fn from(base_denom: &str) -> Self {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(base_denom.as_bytes());
        Self(hash.into())
    }
}
