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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn is_prefixed_by(&self, prefix: &str) -> bool {
        self.prefix.starts_with(prefix)
    }

    #[must_use]
    pub fn denomination_trace(&self) -> String {
        if self.prefix.is_empty() {
            return self.base_denom.clone();
        }

        format!("{}/{}", self.prefix, self.base_denom)
    }

    /// Create a new [`Denom`] with the same base denomination,
    /// but without the prefix of the original.
    #[must_use]
    pub fn to_base_denom(&self) -> Self {
        Self::from_base_denom(&self.base_denom)
    }

    /// Create a new [`Denom`] with the given prefix removed.
    ///
    /// It also ensures the resulting denom does not have a slash at the beginning.
    /// For example, if the denom is `prefix/base`, and the prefix to remove is `prefix`,
    /// then the resulting denom will be `base`.
    ///
    /// # Errors
    ///
    /// - if the denom does not have the given prefix.
    pub fn remove_prefix(&self, prefix: &str) -> Result<Self, InvalidPrefixToRemove> {
        let prefix_to_remove = prefix.trim_end_matches("/");
        if prefix_to_remove == self.prefix {
            return Ok(self.to_base_denom());
        }

        let new_prefix = self
            .prefix
            .clone()
            .strip_prefix(prefix_to_remove)
            .ok_or_else(|| InvalidPrefixToRemove {
                prefix: prefix.to_string(),
                denom: self.to_string(),
            })?
            .trim_start_matches("/")
            .to_string();

        let denom_trace = format!("{new_prefix}/{}", self.base_denom);
        let id = Id::from_denom(&denom_trace);
        Ok(Self {
            id,
            base_denom: self.base_denom.clone(),
            prefix: new_prefix,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("cannot remove prefix {prefix} from {denom}")]
pub struct InvalidPrefixToRemove {
    prefix: String,
    denom: String,
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

/// Asset ID, which is the hash of the denomination trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Id(
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serde::base64"))] [u8; 32],
);

impl Id {
    #[must_use]
    pub fn get(self) -> [u8; 32] {
        self.0
    }

    #[must_use]
    pub fn from_denom(denom: &str) -> Self {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(denom.as_bytes());
        Self(hash.into())
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

impl From<[u8; 32]> for Id {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn remove_prefix_entire_prefix_ok() {
        let denom = Denom::from("prefix/base".to_string());
        let new_denom = denom.remove_prefix("prefix").unwrap();
        assert_eq!(new_denom.to_string(), "base");
        let expected_id = Denom::from_base_denom("base").id();
        assert_eq!(new_denom.id(), expected_id);
    }

    #[test]
    fn remove_prefix_entire_prefix_with_slash_ok() {
        let denom = Denom::from("prefix/base".to_string());
        let new_denom = denom.remove_prefix("prefix/").unwrap();
        assert_eq!(new_denom.to_string(), "base");
        let expected_id = Denom::from_base_denom("base").id();
        assert_eq!(new_denom.id(), expected_id);
    }

    #[test]
    fn remove_prefix_partial_prefix_ok() {
        let denom = Denom::from("prefix-0/prefix-1/base".to_string());
        let new_denom = denom.remove_prefix("prefix-0").unwrap();
        assert_eq!(new_denom.to_string(), "prefix-1/base");
        let expected_id = Denom::from("prefix-1/base".to_string()).id();
        assert_eq!(new_denom.id(), expected_id);
    }

    #[test]
    fn remove_prefix_partial_prefix_with_slash_ok() {
        let denom = Denom::from("prefix-0/prefix-1/base".to_string());
        // adding the slash at the end of the prefix to remove results in the same as without
        let new_denom = denom.remove_prefix("prefix-0/").unwrap();
        assert_eq!(new_denom.to_string(), "prefix-1/base");
        let expected_id = Denom::from("prefix-1/base".to_string()).id();
        assert_eq!(new_denom.id(), expected_id);
    }

    #[test]
    fn remove_prefix_invalid_prefix() {
        let denom = Denom::from("prefix-0/prefix-1/base".to_string());
        let result = denom.remove_prefix("prefix-2");
        assert!(result.is_err());
    }

    #[test]
    fn remove_prefix_cannot_remove_base_denom() {
        let denom = Denom::from("prefix-0/base".to_string());
        let result = denom.remove_prefix("prefix-0/b");
        assert!(result.is_err());
    }
}
