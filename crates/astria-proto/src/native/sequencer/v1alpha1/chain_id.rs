use std::{
    error::Error,
    fmt::Display,
};

pub const CHAIN_ID_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ChainId {
    #[cfg_attr(feature = "serde", serde(with = "hex::serde"))]
    inner: [u8; 32],
}

impl ChainId {
    /// Creates a new `ChainId` from a 32 byte array.
    ///
    /// Use this if you already have a 32 byte array. Prefer
    /// [`ChainId::with_unhashed_bytes`] if you have a clear text
    /// name what you want to use to identify your rollup.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let bytes = [42u8; 32];
    /// let chain_id = ChainId::new(bytes);
    /// assert_eq!(bytes, chain_id.get());
    /// ```
    #[must_use]
    pub fn new(inner: [u8; CHAIN_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Returns the 32 bytes array representing the chain ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let bytes = [42u8; 32];
    /// let chain_id = ChainId::new(bytes);
    /// assert_eq!(bytes, chain_id.get());
    /// ```
    #[must_use]
    pub fn get(self) -> [u8; 32] {
        self.inner
    }

    /// Creates a new `ChainId` by applying Sha256 to `bytes`.
    ///
    /// Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// use sha2::{
    ///     Digest,
    ///     Sha256,
    /// };
    /// let name = "MyRollup-1";
    /// let hashed = Sha256::digest(name);
    /// let chain_id = ChainId::with_unhashed_bytes(name);
    /// assert_eq!(chain_id, ChainId::new(hashed.into()));
    /// ```
    #[must_use]
    pub fn with_unhashed_bytes<T: AsRef<[u8]>>(bytes: T) -> Self {
        use sha2::{
            Digest as _,
            Sha256,
        };
        Self {
            inner: Sha256::digest(bytes).into(),
        }
    }

    /// Allocates a vector from the fixed size array holding the chain ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let chain_id = ChainId::new([42u8; 32]);
    /// assert_eq!(vec![42u8; 32], chain_id.to_vec());
    /// ```
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    /// Convert a byte slice to a chain ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectChainIdLength> {
        let inner = <[u8; CHAIN_ID_LEN]>::try_from(bytes).map_err(|_| IncorrectChainIdLength {
            received: bytes.len(),
        })?;
        Ok(Self::new(inner))
    }
}

impl AsRef<[u8]> for ChainId {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<[u8; CHAIN_ID_LEN]> for ChainId {
    fn from(inner: [u8; CHAIN_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.inner {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct IncorrectChainIdLength {
    received: usize,
}

impl Display for IncorrectChainIdLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 32 bytes, got {}", self.received)
    }
}

impl Error for IncorrectChainIdLength {}
