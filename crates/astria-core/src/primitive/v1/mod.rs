pub mod asset;
pub mod u128;

pub use astria_core_address::{
    Address,
    Bech32,
    Bech32m,
    Builder as AddressBuilder,
    Error as AddressError,
    Format,
    ADDRESS_LENGTH as ADDRESS_LEN,
};
use base64::{
    display::Base64Display,
    prelude::BASE64_URL_SAFE,
};
use bytes::Bytes;
use sha2::{
    Digest as _,
    Sha256,
};

use crate::{
    generated::astria::primitive::v1 as raw,
    Protobuf,
};

pub const ROLLUP_ID_LEN: usize = 32;

pub const TRANSACTION_ID_LEN: usize = 32;

impl Protobuf for Address<Bech32m> {
    type Error = AddressError;
    type Raw = raw::Address;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::Address {
            bech32m,
        } = raw;
        bech32m.parse()
    }

    fn to_raw(&self) -> Self::Raw {
        raw::Address {
            bech32m: self.to_string(),
        }
    }
}

impl Protobuf for merkle::Proof {
    type Error = merkle::audit::InvalidProof;
    type Raw = raw::Proof;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        // XXX: Implementing this by cloning is ok because `audit_path`
        //      has to be cloned always due to `UncheckedProof`'s constructor.
        Self::try_from_raw(raw.clone())
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            audit_path,
            leaf_index,
            tree_size,
        } = raw;
        let leaf_index = leaf_index.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        let tree_size = tree_size.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        Self::unchecked()
            .audit_path(audit_path.to_vec())
            .leaf_index(leaf_index)
            .tree_size(tree_size)
            .try_into_proof()
    }

    fn to_raw(&self) -> Self::Raw {
        // XXX: Implementing in terms of clone is ok because the fields would need to be cloned
        // anyway.
        self.clone().into_raw()
    }

    fn into_raw(self) -> Self::Raw {
        let merkle::audit::UncheckedProof {
            audit_path,
            leaf_index,
            tree_size,
        } = self.into_unchecked();
        Self::Raw {
            audit_path: audit_path.into(),
            leaf_index: leaf_index.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
            tree_size: tree_size.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct RollupId {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serde::base64_serialize")
    )]
    inner: [u8; 32],
}

impl RollupId {
    /// Creates a new rollup ID from a 32 byte array.
    ///
    /// Use this if you already have a 32 byte array. Prefer
    /// [`RollupId::from_unhashed_bytes`] if you have a clear text
    /// name what you want to use to identify your rollup.
    ///
    /// # Examples
    /// ```
    /// use astria_core::primitive::v1::RollupId;
    /// let bytes = [42u8; 32];
    /// let rollup_id = RollupId::new(bytes);
    /// assert_eq!(bytes, *rollup_id.as_bytes());
    /// ```
    #[must_use]
    pub const fn new(inner: [u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Returns a ref to the 32 bytes array representing the rollup ID.
    ///
    /// # Examples
    /// ```
    /// use astria_core::primitive::v1::RollupId;
    /// let bytes = [42u8; 32];
    /// let rollup_id = RollupId::new(bytes);
    /// assert_eq!(bytes, *rollup_id.as_bytes());
    /// ```
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.inner
    }

    /// Creates a new rollup ID by applying Sha256 to `bytes`.
    ///
    /// Examples
    /// ```
    /// use astria_core::primitive::v1::RollupId;
    /// use sha2::{
    ///     Digest,
    ///     Sha256,
    /// };
    /// let name = "MyRollup-1";
    /// let hashed = Sha256::digest(name);
    /// let rollup_id = RollupId::from_unhashed_bytes(name);
    /// assert_eq!(rollup_id, RollupId::new(hashed.into()));
    /// ```
    #[must_use]
    pub fn from_unhashed_bytes<T: AsRef<[u8]>>(bytes: T) -> Self {
        Self {
            inner: Sha256::digest(bytes).into(),
        }
    }

    /// Allocates a vector from the fixed size array holding the rollup ID.
    ///
    /// # Examples
    /// ```
    /// use astria_core::primitive::v1::RollupId;
    /// let rollup_id = RollupId::new([42u8; 32]);
    /// assert_eq!(vec![42u8; 32], rollup_id.to_vec());
    /// ```
    #[must_use]
    pub fn to_vec(self) -> Vec<u8> {
        self.inner.to_vec()
    }

    /// Convert a byte slice to a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectRollupIdLength> {
        let inner =
            <[u8; ROLLUP_ID_LEN]>::try_from(bytes).map_err(|_| IncorrectRollupIdLength {
                received: bytes.len(),
            })?;
        Ok(Self::new(inner))
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::RollupId {
        raw::RollupId {
            inner: self.to_vec().into(),
        }
    }

    #[must_use]
    pub fn into_raw(self) -> raw::RollupId {
        raw::RollupId {
            inner: self.to_vec().into(),
        }
    }

    /// Converts from protobuf type to rust type for a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "for symmetry with other domain type conversions"
    )]
    pub fn try_from_raw(raw: raw::RollupId) -> Result<Self, IncorrectRollupIdLength> {
        Self::try_from_raw_ref(&raw)
    }

    /// Converts from protobuf type to rust type for a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_raw_ref(raw: &raw::RollupId) -> Result<Self, IncorrectRollupIdLength> {
        Self::try_from_slice(&raw.inner)
    }
}

impl AsRef<[u8]> for RollupId {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<[u8; ROLLUP_ID_LEN]> for RollupId {
    fn from(inner: [u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }
}

impl From<&[u8; ROLLUP_ID_LEN]> for RollupId {
    fn from(inner: &[u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner: *inner,
        }
    }
}

impl From<&RollupId> for RollupId {
    fn from(value: &RollupId) -> Self {
        *value
    }
}

impl std::fmt::Display for RollupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Base64Display::new(self.as_ref(), &BASE64_URL_SAFE).fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectRollupIdLength {
    received: usize,
}

/// Derive a [`merkle::Tree`] from an iterable.
///
/// It is the responsibility of the caller to ensure that the iterable is
/// deterministic. Prefer types like `Vec`, `BTreeMap` or `IndexMap` over
/// `HashMap`.
pub fn derive_merkle_tree_from_rollup_txs<'a, T, U>(rollup_ids_to_txs: T) -> merkle::Tree
where
    T: IntoIterator<Item = (&'a RollupId, &'a U)>,
    U: AsRef<[Bytes]> + 'a + ?Sized,
{
    let mut tree = merkle::Tree::new();
    for (rollup_id, txs) in rollup_ids_to_txs {
        let root = merkle::Tree::from_leaves(txs.as_ref()).root();
        tree.build_leaf().write(rollup_id.as_ref()).write(&root);
    }
    tree
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "raw::TransactionId", into = "raw::TransactionId")
)]
pub struct TransactionId {
    inner: [u8; TRANSACTION_ID_LEN],
}

impl TransactionId {
    /// Constructs a new `TransactionId` from a 32-byte array.
    #[must_use]
    pub const fn new(inner: [u8; TRANSACTION_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Consumes `self` and returns the 32-byte transaction hash.
    #[must_use]
    pub fn get(self) -> [u8; TRANSACTION_ID_LEN] {
        self.inner
    }

    /// Returns a reference to the 32-byte transaction hash.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; TRANSACTION_ID_LEN] {
        &self.inner
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::TransactionId {
        raw::TransactionId {
            inner: hex::encode(self.inner),
        }
    }

    #[must_use]
    pub fn into_raw(self) -> raw::TransactionId {
        raw::TransactionId {
            inner: hex::encode(self.inner),
        }
    }

    /// Convert from a reference to raw protobuf type to a rust type for a transaction ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction ID buffer was not 32 bytes long or if it was not hex
    /// encoded.
    pub fn try_from_raw_ref(raw: &raw::TransactionId) -> Result<Self, TransactionIdError> {
        use hex::FromHex as _;

        let inner = <[u8; TRANSACTION_ID_LEN]>::from_hex(&raw.inner).map_err(|err| {
            TransactionIdError(TransactionIdErrorKind::HexDecode {
                source: err,
            })
        })?;
        Ok(Self {
            inner,
        })
    }
}

impl From<TransactionId> for raw::TransactionId {
    fn from(val: TransactionId) -> Self {
        val.into_raw()
    }
}

impl TryFrom<raw::TransactionId> for TransactionId {
    type Error = TransactionIdError;

    fn try_from(value: raw::TransactionId) -> Result<Self, Self::Error> {
        Self::try_from_raw_ref(&value)
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.inner {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionIdError(TransactionIdErrorKind);

#[derive(Debug, thiserror::Error)]
enum TransactionIdErrorKind {
    #[error("error decoding hex string `inner` to bytes")]
    HexDecode { source: hex::FromHexError },
}

#[cfg(test)]
mod tests {
    use super::{
        Address,
        ADDRESS_LEN,
    };
    use crate::Protobuf as _;
    const ASTRIA_ADDRESS_PREFIX: &str = "astria";
    const ASTRIA_COMPAT_ADDRESS_PREFIX: &str = "astriacompat";

    #[cfg(feature = "serde")]
    #[test]
    fn snapshots() {
        use crate::primitive::v1::Bech32;

        let main_address = Address::builder()
            .array([42; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        insta::assert_json_snapshot!(&main_address.to_raw());

        let compat_address = main_address
            .to_prefix(ASTRIA_COMPAT_ADDRESS_PREFIX)
            .unwrap()
            .to_format::<Bech32>();
        // We don't allow serializing non bech32m addresses due to
        // its impl via the protobuf type.
        insta::assert_snapshot!(&compat_address);
    }

    #[test]
    fn can_construct_protobuf_from_address_with_maximally_sized_prefix() {
        // 83 is the maximal length of a hrp
        let long_prefix = [b'a'; 83];
        let address = Address::builder()
            .array([42u8; ADDRESS_LEN])
            .prefix(std::str::from_utf8(&long_prefix).unwrap())
            .try_build()
            .unwrap();
        let _ = address.into_raw();
    }

    #[cfg(feature = "unchecked-constructors")]
    #[test]
    fn address_to_unchecked_roundtrip() {
        let bytes = [42u8; ADDRESS_LEN];
        let input = Address::builder()
            .array(bytes)
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let unchecked = input.into_raw();
        let roundtripped = Address::try_from_raw(unchecked).unwrap();
        assert_eq!(input, roundtripped);
        assert_eq!(input.as_bytes(), roundtripped.as_bytes());
        assert_eq!("astria", input.prefix());
    }
}
