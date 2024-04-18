pub mod asset;
pub mod u128;

use base64::{
    display::Base64Display,
    prelude::BASE64_STANDARD,
};
use sha2::{
    Digest as _,
    Sha256,
};

use crate::{
    generated::primitive::v1 as raw,
    Protobuf,
};

pub const ADDRESS_LEN: usize = 20;
pub const ROLLUP_ID_LEN: usize = 32;

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
            .audit_path(audit_path)
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
            audit_path,
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
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serde::base64"))]
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
    /// assert_eq!(bytes, rollup_id.get());
    /// ```
    #[must_use]
    pub const fn new(inner: [u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Returns the 32 bytes array representing the rollup ID.
    ///
    /// # Examples
    /// ```
    /// use astria_core::primitive::v1::RollupId;
    /// let bytes = [42u8; 32];
    /// let rollup_id = RollupId::new(bytes);
    /// assert_eq!(bytes, rollup_id.get());
    /// ```
    #[must_use]
    pub const fn get(self) -> [u8; 32] {
        self.inner
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

    /// Converts a byte vector to a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_vec(bytes: Vec<u8>) -> Result<Self, IncorrectRollupIdLength> {
        let inner =
            <[u8; ROLLUP_ID_LEN]>::try_from(bytes).map_err(|bytes| IncorrectRollupIdLength {
                received: bytes.len(),
            })?;
        Ok(Self::new(inner))
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
        Base64Display::new(self.as_ref(), &BASE64_STANDARD).fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectRollupIdLength {
    received: usize,
}

/// Indicates that the protobuf response contained an array field that was not 20 bytes long.
#[derive(Debug, thiserror::Error)]
#[error("expected 20 bytes, got {received}")]
pub struct IncorrectAddressLength {
    received: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Address(
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serde::base64"))]
    [u8; ADDRESS_LEN],
);

impl Address {
    #[must_use]
    pub fn get(self) -> [u8; ADDRESS_LEN] {
        self.0
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Construct a sequencer address from a [`ed25519_consensus::VerificationKey`].
    ///
    /// The first 20 bytes of the sha256 hash of the verification key is the address.
    #[must_use]
    // Silence the clippy lint because the function body asserts that the panic
    // cannot happen.
    #[allow(clippy::missing_panics_doc)]
    pub fn from_verification_key(public_key: ed25519_consensus::VerificationKey) -> Self {
        /// this ensures that `ADDRESS_LEN` is never accidentally changed to a value
        /// that would violate this assumption.
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(ADDRESS_LEN <= 32);
        let bytes: [u8; 32] = Sha256::digest(public_key).into();
        Self::try_from_slice(&bytes[..ADDRESS_LEN])
            .expect("can convert 32 byte hash to 20 byte array")
    }

    /// Convert a byte slice to an address.
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer was not 20 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectAddressLength> {
        let inner = <[u8; ADDRESS_LEN]>::try_from(bytes).map_err(|_| IncorrectAddressLength {
            received: bytes.len(),
        })?;
        Ok(Self::from_array(inner))
    }

    #[must_use]
    pub const fn from_array(array: [u8; ADDRESS_LEN]) -> Self {
        Self(array)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; ADDRESS_LEN]> for Address {
    fn from(inner: [u8; ADDRESS_LEN]) -> Self {
        Self(inner)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Base64Display::new(self.as_ref(), &BASE64_STANDARD).fmt(f)
    }
}

/// Derive a [`merkle::Tree`] from an iterable.
///
/// It is the responsbility if the caller to ensure that the iterable is
/// deterministic. Prefer types like `Vec`, `BTreeMap` or `IndexMap` over
/// `HashMap`.
pub fn derive_merkle_tree_from_rollup_txs<'a, T: 'a, U: 'a>(rollup_ids_to_txs: T) -> merkle::Tree
where
    T: IntoIterator<Item = (&'a RollupId, &'a U)>,
    U: AsRef<[Vec<u8>]> + 'a + ?Sized,
{
    let mut tree = merkle::Tree::new();
    for (rollup_id, txs) in rollup_ids_to_txs {
        let root = merkle::Tree::from_leaves(txs.as_ref()).root();
        tree.build_leaf().write(rollup_id.as_ref()).write(&root);
    }
    tree
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;

    use super::{
        Address,
        IncorrectAddressLength,
    };

    #[test]
    fn account_of_20_bytes_is_converted_correctly() {
        let expected = Address([42; 20]);
        let account_vec = expected.0.to_vec();
        let actual = Address::try_from_slice(&account_vec).unwrap();
        assert_eq!(expected, actual);
    }

    #[track_caller]
    fn account_conversion_check(bad_account: &[u8]) {
        let error = Address::try_from_slice(bad_account);
        assert!(
            matches!(error, Err(IncorrectAddressLength { .. })),
            "converting form incorrect sized account succeeded where it should have failed"
        );
    }

    #[test]
    fn account_of_incorrect_length_gives_error() {
        account_conversion_check(&[42; 0]);
        account_conversion_check(&[42; 19]);
        account_conversion_check(&[42; 21]);
        account_conversion_check(&[42; 100]);
    }

    #[test]
    fn snapshots() {
        let address = Address([42; 20]);
        assert_json_snapshot!(address);
    }
}
