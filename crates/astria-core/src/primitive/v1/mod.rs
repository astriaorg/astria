pub mod asset;
pub mod u128;

use base64::{
    display::Base64Display,
    prelude::BASE64_STANDARD,
};
use bytes::Bytes;
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
pub const ASTRIA_ADDRESS_PREFIX: &str = "astria";

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

    fn invalid_prefix(source: bech32::primitives::hrp::Error) -> Self {
        Self(AddressErrorKind::InvalidPrefix {
            source,
        })
    }

    fn fields_are_mutually_exclusive() -> Self {
        Self(AddressErrorKind::FieldsAreMutuallyExclusive)
    }

    fn incorrect_address_length(received: usize) -> Self {
        Self(AddressErrorKind::IncorrectAddressLength {
            received,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum AddressErrorKind {
    #[error("failed decoding provided bech32m string")]
    Bech32mDecode { source: bech32::DecodeError },
    #[error("fields `inner` and `bech32m` are mutually exclusive, only one can be set")]
    FieldsAreMutuallyExclusive,
    #[error("expected an address of 20 bytes, got `{received}`")]
    IncorrectAddressLength { received: usize },
    #[error("the provided prefix was not a valid bech32 human readable prefix")]
    InvalidPrefix {
        source: bech32::primitives::hrp::Error,
    },
}

pub struct NoBytes;
pub struct NoPrefix;
pub struct WithBytes<'a>(BytesInner<'a>);
enum BytesInner<'a> {
    Array([u8; ADDRESS_LEN]),
    Slice(std::borrow::Cow<'a, [u8]>),
}
pub struct WithPrefix<'a>(std::borrow::Cow<'a, str>);

pub struct AddressBuilder<TBytes = NoBytes, TPrefix = NoPrefix> {
    bytes: TBytes,
    prefix: TPrefix,
}

impl AddressBuilder {
    const fn new() -> Self {
        Self {
            bytes: NoBytes,
            prefix: NoPrefix,
        }
    }
}

impl<TBytes, TPrefix> AddressBuilder<TBytes, TPrefix> {
    pub fn array(self, array: [u8; ADDRESS_LEN]) -> AddressBuilder<WithBytes<'static>, TPrefix> {
        AddressBuilder {
            bytes: WithBytes(BytesInner::Array(array)),
            prefix: self.prefix,
        }
    }

    pub fn slice<'a, T: Into<std::borrow::Cow<'a, [u8]>>>(
        self,
        bytes: T,
    ) -> AddressBuilder<WithBytes<'a>, TPrefix> {
        AddressBuilder {
            bytes: WithBytes(BytesInner::Slice(bytes.into())),
            prefix: self.prefix,
        }
    }

    pub fn prefix<'a, T: Into<std::borrow::Cow<'a, str>>>(
        self,
        prefix: T,
    ) -> AddressBuilder<TBytes, WithPrefix<'a>> {
        AddressBuilder {
            bytes: self.bytes,
            prefix: WithPrefix(prefix.into()),
        }
    }
}

impl<'a, 'b> AddressBuilder<WithBytes<'a>, WithPrefix<'b>> {
    pub fn build(self) -> Address {
        self.try_build().unwrap()
    }

    pub fn try_build(self) -> Result<Address, AddressError> {
        let Self {
            bytes: WithBytes(bytes),
            prefix: WithPrefix(prefix),
        } = self;
        let bytes = match bytes {
            BytesInner::Array(bytes) => bytes,
            BytesInner::Slice(bytes) => <[u8; ADDRESS_LEN]>::try_from(bytes.as_ref())
                .map_err(|_| AddressError::incorrect_address_length(bytes.len()))?,
        };
        let prefix = bech32::Hrp::parse(&prefix).map_err(AddressError::invalid_prefix)?;
        Ok(Address {
            bytes,
            prefix,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "raw::Address"))]
pub struct Address {
    prefix: bech32::Hrp,
    bytes: [u8; ADDRESS_LEN],
}

impl Address {
    pub fn builder() -> AddressBuilder {
        AddressBuilder::new()
    }

    // #[must_use]
    // pub fn get(self) -> [u8; ADDRESS_LEN] {
    //     self.0
    // }
    #[must_use]
    pub fn bytes(self) -> [u8; ADDRESS_LEN] {
        self.bytes
    }

    /// Convert a string containing a bech32m string to an astria address.
    ///
    /// # Errors
    /// Returns an error if:
    /// + `input` is not bech32m encoded.
    /// + the decoded data contained in `input` is not 20 bytes long.
    /// + the bech32 hrp prefix exceeds 16 bytes.
    pub fn try_from_bech32m(input: &str) -> Result<Self, AddressError> {
        let (hrp, bytes) = bech32::decode(input).map_err(AddressError::bech32m_decode)?;
        Self::builder()
            .slice(bytes)
            .prefix(hrp.as_str())
            .try_build()
    }

    /// Convert [`Address`] to a [`raw::Address`].
    // allow: panics are checked to not happen
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn to_raw(&self) -> raw::Address {
        let bech32m = bech32::encode_lower::<bech32::Bech32m>(self.prefix, &self.bytes())
            .expect("must not fail because len(prefix) + len(bytes) <= 63 < BECH32M::CODELENGTH");
        // allow: the field is deprecated, but we must still fill it in
        #[allow(deprecated)]
        raw::Address {
            inner: Bytes::new(),
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
        // allow: `Address::inner` field is deprecated, but we must still check it
        #[allow(deprecated)]
        let raw::Address {
            inner,
            bech32m,
        } = raw;
        if bech32m.is_empty() {
            return Self::builder()
                .slice(inner.as_ref())
                .prefix("astria")
                .try_build();
        }
        if inner.is_empty() {
            return Self::try_from_bech32m(bech32m);
        }
        Err(AddressError::fields_are_mutually_exclusive())
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

// impl From<[u8; ADDRESS_LEN]> for Address {
//     fn from(inner: [u8; ADDRESS_LEN]) -> Self {
//         Self(inner)
//     }
// }

impl From<Address> for raw::Address {
    fn from(value: Address) -> Self {
        value.into_raw()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use bech32::EncodeError;
        match bech32::encode_lower_to_fmt::<bech32::Bech32m, _>(f, self.prefix, &self.bytes()) {
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
        raw,
        Address,
        AddressError,
        AddressErrorKind,
        ADDRESS_LEN,
        ASTRIA_ADDRESS_PREFIX,
    };

    #[track_caller]
    fn assert_wrong_address_bytes(bad_account: &[u8]) {
        let error = Address::builder()
            .slice(bad_account)
            .prefix("astria")
            .try_build()
            .expect_err(
                "converting from an incorrectly sized byte slice succeeded where it should have \
                 failed",
            );
        let AddressError(AddressErrorKind::IncorrectAddressLength {
            received,
        }) = error
        else {
            panic!("expected AddressErrorKind::IncorrectAddressLength, got {error:?}");
        };
        assert_eq!(bad_account.len(), received);
    }

    #[test]
    fn account_of_incorrect_length_gives_error() {
        assert_wrong_address_bytes(&[42; 0]);
        assert_wrong_address_bytes(&[42; 19]);
        assert_wrong_address_bytes(&[42; 21]);
        assert_wrong_address_bytes(&[42; 100]);
    }

    #[test]
    fn snapshots() {
        let address = Address::builder().array([42; 20]).prefix("astria").build();
        assert_json_snapshot!(address);
    }

    #[test]
    fn can_construct_protobuf_from_address_with_maximally_sized_prefix() {
        // 83 is the maximal length of a hrp
        let long_prefix = [b'a'; 83];
        let address = Address::builder()
            .array([42u8; ADDRESS_LEN])
            .prefix(std::str::from_utf8(&long_prefix).unwrap())
            .build();
        let _ = address.into_raw();
    }

    #[test]
    fn bech32m_and_deprecated_bytes_field_are_mutually_exclusive() {
        // allow: `Address::inner` field is deprecated, but we must still check it
        #![allow(deprecated)]
        let bytes = Bytes::copy_from_slice(&[24u8; ADDRESS_LEN]);
        let bech32m = [42u8; ADDRESS_LEN];
        let proto = super::raw::Address {
            inner: bytes.clone(),
            bech32m: bech32::encode_lower::<bech32::Bech32m>(
                bech32::Hrp::parse(ASTRIA_ADDRESS_PREFIX).unwrap(),
                &bech32m,
            )
            .unwrap(),
        };
        let expected = AddressErrorKind::FieldsAreMutuallyExclusive;
        let actual = Address::try_from_raw(&proto)
            .expect_err("returned a valid address where it should have errored");
        assert_eq!(expected, actual.0);
    }

    #[test]
    fn proto_with_missing_bech32m_is_accepted_and_assumed_astria() {
        // allow: `Address::inner` field is deprecated, but we must still check it
        #![allow(deprecated)]
        let bytes = [42u8; ADDRESS_LEN];
        let input = raw::Address {
            inner: Bytes::copy_from_slice(&bytes),
            bech32m: String::new(),
        };
        let address = Address::try_from_raw(&input).unwrap();
        assert_eq!("astria", address.prefix.as_str());
        assert_eq!(bytes, address.bytes());
    }

    #[test]
    fn proto_with_missing_bytes_is_accepted() {
        // allow: `Address::inner` field is deprecated, but we must still check it
        #![allow(deprecated)]
        let bytes = [42u8; ADDRESS_LEN];
        let input = raw::Address {
            inner: Bytes::new(),
            bech32m: bech32::encode_lower::<bech32::Bech32m>(
                bech32::Hrp::parse(ASTRIA_ADDRESS_PREFIX).unwrap(),
                &bytes,
            )
            .unwrap(),
        };
        let address = Address::try_from_raw(&input).unwrap();
        assert_eq!(bytes, address.bytes());
    }

    #[test]
    fn protobuf_only_has_bech32m_populated() {
        // allow: `Address::inner` field is deprecated, but we must still check it
        #![allow(deprecated)]
        let bytes = [42u8; ADDRESS_LEN];
        let address = Address::builder().array(bytes).prefix("astria").build();
        let output = address.into_raw();
        assert!(
            output.inner.is_empty(),
            "the deprecated bytes field must not be set"
        );
        assert_eq!(
            bech32::encode_lower::<bech32::Bech32m>(
                bech32::Hrp::parse(ASTRIA_ADDRESS_PREFIX).unwrap(),
                &bytes
            )
            .unwrap(),
            output.bech32m
        );
    }
}
