pub mod asset;
pub mod u128;

use std::{
    marker::PhantomData,
    str::FromStr,
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
    generated::primitive::v1 as raw,
    Protobuf,
};

pub const ADDRESS_LEN: usize = 20;

pub const ROLLUP_ID_LEN: usize = 32;

pub const TRANSACTION_ID_LEN: usize = 32;

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

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AddressError(AddressErrorKind);

impl AddressError {
    fn decode(source: bech32::primitives::decode::CheckedHrpstringError) -> Self {
        Self(AddressErrorKind::Decode {
            source,
        })
    }

    fn invalid_prefix(source: bech32::primitives::hrp::Error) -> Self {
        Self(AddressErrorKind::InvalidPrefix {
            source,
        })
    }

    fn incorrect_address_length(received: usize) -> Self {
        Self(AddressErrorKind::IncorrectAddressLength {
            received,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum AddressErrorKind {
    #[error("failed decoding provided string")]
    Decode {
        source: bech32::primitives::decode::CheckedHrpstringError,
    },
    #[error("expected an address of 20 bytes, got `{received}`")]
    IncorrectAddressLength { received: usize },
    #[error("the provided prefix was not a valid bech32 human readable prefix")]
    InvalidPrefix {
        source: bech32::primitives::hrp::Error,
    },
}

pub struct NoBytes;
pub struct NoPrefix;
pub struct WithBytes<'a, I>(WithBytesInner<'a, I>);
enum WithBytesInner<'a, I> {
    Array([u8; ADDRESS_LEN]),
    Iter(I),
    Slice(std::borrow::Cow<'a, [u8]>),
}
pub struct WithPrefix<'a>(std::borrow::Cow<'a, str>);

pub struct NoBytesIter;

impl Iterator for NoBytesIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct AddressBuilder<TFormat, TBytes = NoBytes, TPrefix = NoPrefix> {
    bytes: TBytes,
    prefix: TPrefix,
    format: PhantomData<TFormat>,
}

impl<TFormat> AddressBuilder<TFormat, NoBytes, NoPrefix> {
    const fn new() -> Self {
        Self {
            bytes: NoBytes,
            prefix: NoPrefix,
            format: PhantomData,
        }
    }
}

impl<TFormat, TBytes, TPrefix> AddressBuilder<TFormat, TBytes, TPrefix> {
    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn array(
        self,
        array: [u8; ADDRESS_LEN],
    ) -> AddressBuilder<TFormat, WithBytes<'static, NoBytesIter>, TPrefix> {
        AddressBuilder {
            bytes: WithBytes(WithBytesInner::Array(array)),
            prefix: self.prefix,
            format: self.format,
        }
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn slice<'a, T: Into<std::borrow::Cow<'a, [u8]>>>(
        self,
        bytes: T,
    ) -> AddressBuilder<TFormat, WithBytes<'a, NoBytesIter>, TPrefix> {
        AddressBuilder {
            bytes: WithBytes(WithBytesInner::Slice(bytes.into())),
            prefix: self.prefix,
            format: self.format,
        }
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn with_iter<T: IntoIterator<Item = u8>>(
        self,
        iter: T,
    ) -> AddressBuilder<TFormat, WithBytes<'static, T>, TPrefix> {
        AddressBuilder {
            bytes: WithBytes(WithBytesInner::Iter(iter)),
            prefix: self.prefix,
            format: self.format,
        }
    }

    /// Use the given verification key for address generation.
    ///
    /// The verification key is hashed with SHA256 and the first 20 bytes are used as the address
    /// bytes.
    #[expect(clippy::missing_panics_doc, reason = "the conversion is infallible")]
    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn verification_key(
        self,
        key: &crate::crypto::VerificationKey,
    ) -> AddressBuilder<TFormat, WithBytes<'static, NoBytesIter>, TPrefix> {
        let hash = Sha256::digest(key.as_bytes());
        let array: [u8; ADDRESS_LEN] = hash[0..ADDRESS_LEN]
            .try_into()
            .expect("hash is 32 bytes long, so must always be able to convert to 20 bytes");
        self.array(array)
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn prefix<'a, T: Into<std::borrow::Cow<'a, str>>>(
        self,
        prefix: T,
    ) -> AddressBuilder<TFormat, TBytes, WithPrefix<'a>> {
        AddressBuilder {
            bytes: self.bytes,
            prefix: WithPrefix(prefix.into()),
            format: self.format,
        }
    }
}

impl<'a, 'b, TFormat, TBytesIter> AddressBuilder<TFormat, WithBytes<'a, TBytesIter>, WithPrefix<'b>>
where
    TBytesIter: IntoIterator<Item = u8>,
{
    /// Attempts to build an address from the configured prefix and bytes.
    ///
    /// # Errors
    /// Returns an error if one of the following conditions are violated:
    /// + if the prefix shorter than 1 or longer than 83 characters, or contains characters outside
    ///   33-126 of ASCII characters.
    /// + if the provided bytes are not exactly 20 bytes.
    pub fn try_build(self) -> Result<Address<TFormat>, AddressError> {
        let Self {
            bytes: WithBytes(bytes),
            prefix: WithPrefix(prefix),
            format,
        } = self;
        let bytes = match bytes {
            WithBytesInner::Array(bytes) => bytes,
            WithBytesInner::Iter(bytes) => try_collect_to_array(bytes)?,
            WithBytesInner::Slice(bytes) => <[u8; ADDRESS_LEN]>::try_from(bytes.as_ref())
                .map_err(|_| AddressError::incorrect_address_length(bytes.len()))?,
        };
        let prefix = bech32::Hrp::parse(&prefix).map_err(AddressError::invalid_prefix)?;
        Ok(Address {
            bytes,
            prefix,
            format,
        })
    }
}

fn try_collect_to_array<I: IntoIterator<Item = u8>>(
    iter: I,
) -> Result<[u8; ADDRESS_LEN], AddressError> {
    let mut arr = [0; ADDRESS_LEN];
    let mut iter = iter.into_iter();
    let mut i = 0;
    loop {
        if i >= ADDRESS_LEN {
            break;
        }
        let Some(byte) = iter.next() else {
            break;
        };
        arr[i] = byte;
        i = i.saturating_add(1);
    }
    let items_in_iterator = i.saturating_add(iter.count());
    if items_in_iterator != ADDRESS_LEN {
        return Err(AddressError::incorrect_address_length(items_in_iterator));
    }
    Ok(arr)
}

#[derive(Clone, Copy, Debug)]
pub enum Bech32m {}
#[derive(Clone, Copy, Debug)]
pub enum Bech32 {}
#[derive(Clone, Copy, Debug)]
pub enum NoFormat {}

pub trait Format: private::Sealed {
    type Checksum: bech32::Checksum;
}

impl Format for Bech32m {
    type Checksum = bech32::Bech32m;
}

impl Format for Bech32 {
    type Checksum = bech32::Bech32;
}

impl Format for NoFormat {
    type Checksum = bech32::NoChecksum;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Bech32m {}
    impl Sealed for super::Bech32 {}
    impl Sealed for super::NoFormat {}
}

#[derive(Debug, Hash)]
pub struct Address<T = Bech32m> {
    bytes: [u8; ADDRESS_LEN],
    prefix: bech32::Hrp,
    format: PhantomData<T>,
}

// The serde impls need to be manually implemented for Address because they
// only work for Address<Bech32m> which cannot be expressed using serde
// attributes.
#[cfg(feature = "serde")]
mod _serde_impls {
    use serde::de::Error as _;
    impl serde::Serialize for super::Address<super::Bech32m> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.to_raw().serialize(serializer)
        }
    }
    impl<'de> serde::Deserialize<'de> for super::Address<super::Bech32m> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            super::raw::Address::deserialize(deserializer)
                .and_then(|raw| raw.try_into().map_err(D::Error::custom))
        }
    }
}

impl<TFormat> Clone for Address<TFormat> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<TFormat> Copy for Address<TFormat> {}

impl<TFormat> PartialEq for Address<TFormat> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.eq(&other.bytes) && self.prefix.eq(&other.prefix)
    }
}

impl<TFormat> Eq for Address<TFormat> {}

impl<TFormat> Address<TFormat> {
    #[must_use = "the builder must be used to construct an address to be useful"]
    pub fn builder() -> AddressBuilder<TFormat> {
        AddressBuilder::<TFormat>::new()
    }

    #[must_use]
    pub fn bytes(self) -> [u8; ADDRESS_LEN] {
        self.bytes
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; ADDRESS_LEN] {
        &self.bytes
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        self.prefix.as_str()
    }

    /// Converts to a new address with the given `prefix`.
    ///
    /// # Errors
    /// Returns an error if an address with `prefix` cannot be constructed.
    /// The error conditions for this are the same as for [`AddressBuilder::try_build`].
    pub fn to_prefix(&self, prefix: &str) -> Result<Self, AddressError> {
        Self::builder()
            .array(*self.as_bytes())
            .prefix(prefix)
            .try_build()
    }

    /// Converts to a new address with the type argument `OtherFormat`.
    ///
    /// `OtherFormat` is usually [`Bech32`] or [`Bech32m`].
    #[must_use]
    pub fn to_format<OtherFormat>(&self) -> Address<OtherFormat> {
        Address {
            bytes: self.bytes,
            prefix: self.prefix,
            format: PhantomData,
        }
    }
}

impl Address<Bech32m> {
    /// Convert [`Address`] to a [`raw::Address`].
    #[expect(
        clippy::missing_panics_doc,
        reason = "panics are checked to not happen"
    )]
    #[must_use]
    pub fn to_raw(&self) -> raw::Address {
        let bech32m =
            bech32::encode_lower::<<Bech32m as Format>::Checksum>(self.prefix, self.as_bytes())
                .expect(
                    "should not fail because len(prefix) + len(bytes) <= 63 < BECH32M::CODELENGTH",
                );
        raw::Address {
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
        let raw::Address {
            bech32m,
        } = raw;
        bech32m.parse()
    }

    /// This should only be used where the inputs have been provided by a trusted entity, e.g. read
    /// from our own state store.
    ///
    /// Note that this function is not considered part of the public API and is subject to breaking
    /// change at any time.
    #[cfg(feature = "unchecked-constructors")]
    #[doc(hidden)]
    #[must_use]
    pub fn unchecked_from_parts(bytes: [u8; ADDRESS_LEN], prefix: &str) -> Self {
        Self {
            bytes,
            prefix: bech32::Hrp::parse_unchecked(prefix),
            format: PhantomData,
        }
    }
}

impl From<Address<Bech32m>> for raw::Address {
    fn from(value: Address<Bech32m>) -> Self {
        value.into_raw()
    }
}

impl<T: Format> FromStr for Address<T> {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let checked = bech32::primitives::decode::CheckedHrpstring::new::<T::Checksum>(s)
            .map_err(Self::Err::decode)?;
        let hrp = checked.hrp();
        Self::builder()
            .with_iter(checked.byte_iter())
            .prefix(hrp.as_str())
            .try_build()
    }
}

impl TryFrom<raw::Address> for Address<Bech32m> {
    type Error = AddressError;

    fn try_from(value: raw::Address) -> Result<Self, Self::Error> {
        Self::try_from_raw(&value)
    }
}

impl<T: Format> std::fmt::Display for Address<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use bech32::EncodeError;
        match bech32::encode_lower_to_fmt::<T::Checksum, _>(f, self.prefix, self.as_bytes()) {
            Ok(()) => Ok(()),
            Err(EncodeError::Fmt(err)) => Err(err),
            Err(err) => panic!(
                "only formatting errors are valid when encoding astria addresses; all other error \
                 variants (only TooLong as of bech32-0.11.0) are guaranteed to not happen because \
                 `Address` is length checked:\n{err:?}",
            ),
        }
    }
}
/// Constructs a dummy address from a given `prefix`, otherwise fail.
pub(crate) fn try_construct_dummy_address_from_prefix<T: Format>(
    prefix: &str,
) -> Result<(), AddressError> {
    Address::<T::Checksum>::builder()
        .array([0u8; ADDRESS_LEN])
        .prefix(prefix)
        .try_build()
        .map(|_| ())
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
        AddressError,
        AddressErrorKind,
        Bech32m,
        ADDRESS_LEN,
    };
    use crate::primitive::v1::Bech32;
    const ASTRIA_ADDRESS_PREFIX: &str = "astria";
    const ASTRIA_COMPAT_ADDRESS_PREFIX: &str = "astriacompat";

    #[track_caller]
    fn assert_wrong_address_bytes(bad_account: &[u8]) {
        let error = Address::<Bech32m>::builder()
            .slice(bad_account)
            .prefix(ASTRIA_ADDRESS_PREFIX)
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
    fn parse_bech32m_address() {
        let expected = Address::builder()
            .array([42; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let actual = expected.to_string().parse::<Address>().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_bech32_address() {
        let expected = Address::<Bech32>::builder()
            .array([42; 20])
            .prefix(ASTRIA_COMPAT_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let actual = expected.to_string().parse::<Address<Bech32>>().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn parsing_bech32_address_as_bech32m_fails() {
        let expected = Address::<Bech32>::builder()
            .array([42; 20])
            .prefix(ASTRIA_COMPAT_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let err = expected
            .to_string()
            .parse::<Address<Bech32m>>()
            .expect_err("this must not work");
        match err {
            AddressError(AddressErrorKind::Decode {
                ..
            }) => {}
            other => {
                panic!(
                    "expected AddressError(AddressErrorKind::Decode {{ .. }}), but got {other:?}"
                )
            }
        }
    }

    #[test]
    fn parsing_bech32m_address_as_bech32_fails() {
        let expected = Address::<Bech32m>::builder()
            .array([42; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let err = expected
            .to_string()
            .parse::<Address<Bech32>>()
            .expect_err("this must not work");
        match err {
            AddressError(AddressErrorKind::Decode {
                ..
            }) => {}
            other => {
                panic!(
                    "expected AddressError(AddressErrorKind::Decode {{ .. }}), but got {other:?}"
                )
            }
        }
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
        let roundtripped = Address::try_from_raw(&unchecked).unwrap();
        assert_eq!(input, roundtripped);
        assert_eq!(input.as_bytes(), roundtripped.as_bytes());
        assert_eq!("astria", input.prefix());
    }
}
