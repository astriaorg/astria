pub mod asset;
pub mod u128;

use base64::{
    display::Base64Display,
    prelude::{
        Engine as _,
        BASE64_STANDARD,
    },
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
/// The human readable prefix of astria addresses (also known as bech32 HRP).
pub const HUMAN_READABLE_ADDRESS_PREFIX: &str = "astria";
// The compile-time generated bech32::Hrp to avoid redoing it on every encode.
// Intentionally kept crate-private to not make bech32 part of the crate API.
const BECH32_HRP: bech32::Hrp = bech32::Hrp::parse_unchecked(HUMAN_READABLE_ADDRESS_PREFIX);
// The length of astria addresses as bech32m strings.
const ADDRESS_BECH32M_LENGTH: usize = 45;

pub const ROLLUP_ID_LEN: usize = 32;
pub const FEE_ASSET_ID_LEN: usize = 32;

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
    pub fn try_from_raw(raw: &raw::RollupId) -> Result<Self, IncorrectRollupIdLength> {
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
        Base64Display::new(self.as_ref(), &BASE64_STANDARD).fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectRollupIdLength {
    received: usize,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AddressError(AddressErrorKind);

impl AddressError {
    fn bech32m_decode(source: bech32::DecodeError) -> Self {
        Self(AddressErrorKind::Bech32mDecode {
            source,
        })
    }

    fn fields_dont_match(bytes: [u8; ADDRESS_LEN], bech32m: [u8; ADDRESS_LEN]) -> Self {
        Self(AddressErrorKind::FieldsDontMatch {
            bytes,
            bech32m,
        })
    }

    fn incorrect_address_length(received: usize) -> Self {
        Self(AddressErrorKind::IncorrectAddressLength {
            received,
        })
    }

    fn unknown_bech32_hrp(received: bech32::Hrp) -> Self {
        Self(AddressErrorKind::UnknownBech32Hrp {
            received,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum AddressErrorKind {
    #[error("failed decoding provided bech32m string")]
    Bech32mDecode { source: bech32::DecodeError },
    #[error(
        "address protobuf contained mismatched address in its `bytes` ({}) and `bech32m` ({}) fields",
        BASE64_STANDARD.encode(.bytes),
        BASE64_STANDARD.encode(.bech32m),
        )]
    FieldsDontMatch {
        bytes: [u8; ADDRESS_LEN],
        bech32m: [u8; ADDRESS_LEN],
    },
    #[error("expected an address of 20 bytes, got `{received}`")]
    IncorrectAddressLength { received: usize },
    #[error("expected `\"astria\"` as the bech32 human readable prefix, got `\"{received}\"`")]
    UnknownBech32Hrp { received: bech32::Hrp },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "raw::Address"))]
pub struct Address([u8; ADDRESS_LEN]);

impl Address {
    #[must_use]
    pub fn get(self) -> [u8; ADDRESS_LEN] {
        self.0
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Convert a byte slice to an address.
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer was not 20 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AddressError> {
        let inner = <[u8; ADDRESS_LEN]>::try_from(bytes)
            .map_err(|_| AddressError::incorrect_address_length(bytes.len()))?;
        Ok(Self::from_array(inner))
    }

    /// Convert a string containing a bech32m string to an astria address.
    ///
    /// # Errors
    /// Returns an error if:
    /// + `input` is not bech32m encoded.
    /// + the human readable prefix (bech32 HRP) is not `"astria"`.
    /// + the decoded data contained in `input` is not 20 bytes long.
    pub fn try_from_bech32m(input: &str) -> Result<Self, AddressError> {
        let (hrp, bytes) = bech32::decode(input).map_err(AddressError::bech32m_decode)?;
        if hrp != BECH32_HRP {
            return Err(AddressError::unknown_bech32_hrp(hrp));
        }
        Self::try_from_slice(&bytes)
    }

    #[must_use]
    pub const fn from_array(array: [u8; ADDRESS_LEN]) -> Self {
        Self(array)
    }

    /// Convert [`Address`] to a [`raw::Address`].
    // allow: panics are checked to not happen
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn to_raw(&self) -> raw::Address {
        let mut bech32m = String::with_capacity(ADDRESS_BECH32M_LENGTH);
        bech32::encode_lower_to_fmt::<bech32::Bech32m, _>(&mut bech32m, BECH32_HRP, &self.get())
            .expect(
                "must not fail because Address is tested to be ADDRESS_BECH32M_LENGTH long, which \
                 is less than the permitted maximum bech32m checksum length",
            );
        // allow: for compatibility purposes. The `bytes` protobuf field is deprecated
        // and should not be used by downstream users in new code.
        #[allow(deprecated)]
        raw::Address {
            inner: self.to_vec().into(),
            bech32m,
        }
    }

    #[must_use]
    pub fn into_raw(self) -> raw::Address {
        self.to_raw()
    }

    /// Convert from protobuf to rust type an address.
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer was not 20 bytes long.
    pub fn try_from_raw(raw: &raw::Address) -> Result<Self, AddressError> {
        // allow: for compatibility purposes. The `bytes` protobuf field is deprecated
        // and should not be used by downstream users in new code.
        #![allow(deprecated)]
        let raw::Address {
            inner,
            bech32m,
        } = raw;
        if bech32m.is_empty() {
            return Self::try_from_slice(inner);
        }
        if inner.is_empty() {
            return Self::try_from_bech32m(bech32m);
        }
        let addr_bytes = Self::try_from_slice(inner)?;
        let addr_bech32m = Self::try_from_bech32m(bech32m)?;
        if addr_bytes != addr_bech32m {
            return Err(AddressError::fields_dont_match(
                addr_bytes.get(),
                addr_bech32m.get(),
            ));
        }
        Ok(addr_bytes)
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

impl From<Address> for raw::Address {
    fn from(value: Address) -> Self {
        value.into_raw()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use bech32::EncodeError;
        match bech32::encode_lower_to_fmt::<bech32::Bech32m, _>(f, BECH32_HRP, &self.get()) {
            Ok(()) => Ok(()),
            Err(EncodeError::Fmt(err)) => Err(err),
            Err(err) => panic!(
                "only formatting errors are valid when encoding astria addresses; all other error \
                 variants (only TooLong at of bech32-0.11.0) are guaranteed to not \
                 happen:\n{err:?}",
            ),
        }
    }
}

/// Derive a [`merkle::Tree`] from an iterable.
///
/// It is the responsibility of the caller to ensure that the iterable is
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
    use bytes::Bytes;
    use insta::assert_json_snapshot;

    use super::{
        Address,
        AddressErrorKind,
        ADDRESS_LEN,
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
        Address::try_from_slice(bad_account).expect_err(
            "converting from an incorrectly sized byte slice succeeded where it should have failed",
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

    #[test]
    fn const_astria_hrp_is_valid() {
        let hrp = bech32::Hrp::parse(super::HUMAN_READABLE_ADDRESS_PREFIX).unwrap();
        assert_eq!(hrp, super::BECH32_HRP);
    }

    #[test]
    fn const_astria_bech32m_is_correct_length() {
        let actual = bech32::encoded_length::<bech32::Bech32m>(
            super::BECH32_HRP,
            &[42u8; super::ADDRESS_LEN],
        )
        .unwrap();
        assert_eq!(super::ADDRESS_BECH32M_LENGTH, actual);
    }

    #[test]
    fn mismatched_fields_in_protobuf_address_are_caught() {
        // allow: deprecated code must still be tested
        #![allow(deprecated)]
        let bytes = [24u8; ADDRESS_LEN];
        let bech32m = [42u8; ADDRESS_LEN];
        let proto = super::raw::Address {
            inner: Bytes::copy_from_slice(&bytes),
            bech32m: bech32::encode_lower::<bech32::Bech32m>(super::BECH32_HRP, &bech32m).unwrap(),
        };
        let expected = AddressErrorKind::FieldsDontMatch {
            bytes,
            bech32m,
        };
        let actual = Address::try_from_raw(&proto)
            .expect_err("returned a valid address where it should have errored");
        assert_eq!(expected, actual.0);
    }

    #[test]
    fn non_astria_hrp_is_caught() {
        let hrp = bech32::Hrp::parse("notastria").unwrap();
        let input = bech32::encode_lower::<bech32::Bech32m>(hrp, &[42u8; ADDRESS_LEN]).unwrap();
        let actual = Address::try_from_bech32m(&input)
            .expect_err("returned a valid address where it should have errored");
        let expected = AddressErrorKind::UnknownBech32Hrp {
            received: hrp,
        };
        assert_eq!(expected, actual.0);
    }
}
