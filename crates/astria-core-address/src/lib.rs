use std::{
    marker::PhantomData,
    str::FromStr,
};

pub use astria_core_consts::ADDRESS_LENGTH;

#[derive(Debug, Hash)]
pub struct Address<T = Bech32m> {
    bytes: [u8; ADDRESS_LENGTH],
    prefix: bech32::Hrp,
    format: PhantomData<T>,
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
    pub fn builder() -> Builder<TFormat> {
        Builder::new()
    }

    #[must_use]
    pub fn bytes(self) -> [u8; ADDRESS_LENGTH] {
        self.bytes
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
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
    pub fn to_prefix(&self, prefix: &str) -> Result<Self, Error> {
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
    /// Should only be used where the inputs have been provided by a trusted entity, e.g. read
    /// from our own state store.
    ///
    /// Note that this function is not considered part of the public API and is subject to breaking
    /// change at any time.
    #[cfg(feature = "unchecked-constructor")]
    #[doc(hidden)]
    #[must_use]
    pub fn unchecked_from_parts(bytes: [u8; ADDRESS_LENGTH], prefix: &str) -> Self {
        Self {
            bytes,
            prefix: bech32::Hrp::parse_unchecked(prefix),
            format: PhantomData,
        }
    }
}

impl<T: Format> FromStr for Address<T> {
    type Err = Error;

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

pub struct NoBytes;
pub struct NoPrefix;
pub struct WithBytes<'a, I>(WithBytesInner<'a, I>);
enum WithBytesInner<'a, I> {
    Array([u8; ADDRESS_LENGTH]),
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

impl ExactSizeIterator for NoBytesIter {
    fn len(&self) -> usize {
        0
    }
}

pub struct Builder<TFormat, TBytes = NoBytes, TPrefix = NoPrefix> {
    bytes: TBytes,
    prefix: TPrefix,
    format: PhantomData<TFormat>,
}

impl<TFormat> Builder<TFormat, NoBytes, NoPrefix> {
    const fn new() -> Self {
        Self {
            bytes: NoBytes,
            prefix: NoPrefix,
            format: PhantomData,
        }
    }
}

impl<TFormat, TBytes, TPrefix> Builder<TFormat, TBytes, TPrefix> {
    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn array(
        self,
        array: [u8; ADDRESS_LENGTH],
    ) -> Builder<TFormat, WithBytes<'static, NoBytesIter>, TPrefix> {
        Builder {
            bytes: WithBytes(WithBytesInner::Array(array)),
            prefix: self.prefix,
            format: self.format,
        }
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn slice<'a, T: Into<std::borrow::Cow<'a, [u8]>>>(
        self,
        bytes: T,
    ) -> Builder<TFormat, WithBytes<'a, NoBytesIter>, TPrefix> {
        Builder {
            bytes: WithBytes(WithBytesInner::Slice(bytes.into())),
            prefix: self.prefix,
            format: self.format,
        }
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    fn with_iter<T>(self, iter: T) -> Builder<TFormat, WithBytes<'static, T>, TPrefix>
    where
        T: IntoIterator<Item = u8>,
        T::IntoIter: ExactSizeIterator,
    {
        Builder {
            bytes: WithBytes(WithBytesInner::Iter(iter)),
            prefix: self.prefix,
            format: self.format,
        }
    }

    #[must_use = "the builder must be built to construct an address to be useful"]
    pub fn prefix<'a, T: Into<std::borrow::Cow<'a, str>>>(
        self,
        prefix: T,
    ) -> Builder<TFormat, TBytes, WithPrefix<'a>> {
        Builder {
            bytes: self.bytes,
            prefix: WithPrefix(prefix.into()),
            format: self.format,
        }
    }
}

impl<TFormat, TBytesIter> Builder<TFormat, WithBytes<'_, TBytesIter>, WithPrefix<'_>>
where
    TBytesIter: IntoIterator<Item = u8>,
    TBytesIter::IntoIter: ExactSizeIterator,
{
    /// Attempts to build an address from the configured prefix and bytes.
    ///
    /// # Errors
    /// Returns an error if one of the following conditions are violated:
    /// + if the prefix shorter than 1 or longer than 83 characters, or contains characters outside
    ///   33-126 of ASCII characters.
    /// + if the provided bytes are not exactly 20 bytes.
    pub fn try_build(self) -> Result<Address<TFormat>, Error> {
        let Self {
            bytes: WithBytes(bytes),
            prefix: WithPrefix(prefix),
            format,
        } = self;
        let bytes = match bytes {
            WithBytesInner::Array(bytes) => bytes,
            WithBytesInner::Iter(bytes) => try_collect_to_array(bytes)?,
            WithBytesInner::Slice(bytes) => <[u8; ADDRESS_LENGTH]>::try_from(bytes.as_ref())
                .map_err(|_| Error::incorrect_length(bytes.len()))?,
        };
        let prefix = bech32::Hrp::parse(&prefix).map_err(Error::invalid_prefix)?;
        Ok(Address {
            bytes,
            prefix,
            format,
        })
    }
}

fn try_collect_to_array<I>(iter: I) -> Result<[u8; ADDRESS_LENGTH], Error>
where
    I: IntoIterator<Item = u8>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = iter.into_iter();

    if iter.len() != ADDRESS_LENGTH {
        return Err(Error::incorrect_length(iter.len()));
    }
    let mut arr = [0; ADDRESS_LENGTH];
    for (left, right) in arr.iter_mut().zip(iter) {
        *left = right;
    }
    Ok(arr)
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn decode(source: bech32::primitives::decode::CheckedHrpstringError) -> Self {
        Self(ErrorKind::Decode {
            source,
        })
    }

    fn invalid_prefix(source: bech32::primitives::hrp::Error) -> Self {
        Self(ErrorKind::InvalidPrefix {
            source,
        })
    }

    fn incorrect_length(received: usize) -> Self {
        Self(ErrorKind::IncorrectLength {
            received,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum ErrorKind {
    #[error("failed decoding provided string")]
    Decode {
        source: bech32::primitives::decode::CheckedHrpstringError,
    },
    #[error("expected an address of 20 bytes, got `{received}`")]
    IncorrectLength { received: usize },
    #[error("the provided prefix was not a valid bech32 human readable prefix")]
    InvalidPrefix {
        source: bech32::primitives::hrp::Error,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum Bech32m {}
#[derive(Clone, Copy, Debug)]
pub enum Bech32 {}
#[derive(Clone, Copy, Debug)]
pub enum NoFormat {}

#[expect(
    private_bounds,
    reason = "prevent downstream implementation of this trait"
)]
pub trait Format: Sealed {
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

trait Sealed {}
impl Sealed for Bech32m {}
impl Sealed for Bech32 {}
impl Sealed for NoFormat {}

#[cfg(test)]
mod tests {
    use super::{
        Address,
        Bech32,
        Bech32m,
        Error,
        ErrorKind,
    };

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
        let Error(ErrorKind::IncorrectLength {
            received,
        }) = error
        else {
            panic!("expected ErrorKind::IncorrectLength, got {error:?}");
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
            Error(ErrorKind::Decode {
                ..
            }) => {}
            other => {
                panic!("expected Error(ErrorKind::Decode {{ .. }}), but got {other:?}")
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
            Error(ErrorKind::Decode {
                ..
            }) => {}
            other => {
                panic!("expected Error(ErrorKind::Decode {{ .. }}), but got {other:?}")
            }
        }
    }
}
