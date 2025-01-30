use std::fmt::{
    self,
    Debug,
    Display,
    Formatter,
};

use base64::{
    display::Base64Display,
    engine::general_purpose::STANDARD,
};
use thiserror::Error;

/// A SHA256 digest of a Borsh-encoded upgrade change.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChangeHash([u8; 32]);

impl ChangeHash {
    pub const LENGTH: usize = 32;

    #[must_use]
    pub const fn new(digest: [u8; Self::LENGTH]) -> Self {
        Self(digest)
    }

    #[must_use]
    pub fn get(self) -> [u8; Self::LENGTH] {
        self.0
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}

impl Display for ChangeHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Base64Display::new(&self.0, &STANDARD))
    }
}

impl Debug for ChangeHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ChangeHash({self})")
    }
}

impl TryFrom<&[u8]> for ChangeHash {
    type Error = ChangeHashError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let hash = <[u8; Self::LENGTH]>::try_from(bytes).map_err(|_| {
            ChangeHashError(ChangeHashErrorKind::InvalidChangeHashLength {
                expected: Self::LENGTH,
                actual: bytes.len(),
            })
        })?;
        Ok(Self(hash))
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
#[expect(
    clippy::module_name_repetitions,
    reason = "following our naming conventions"
)]
pub struct ChangeHashError(ChangeHashErrorKind);

#[derive(Debug, Error)]
enum ChangeHashErrorKind {
    #[error("upgrade change hash must be {expected} bytes, but {actual} bytes provided")]
    InvalidChangeHashLength { expected: usize, actual: usize },
}
