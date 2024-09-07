use std::{
    cmp::Ordering,
    fmt::{
        self,
        Debug,
        Display,
        Formatter,
    },
    hash::{
        Hash,
        Hasher,
    },
    sync::OnceLock,
};

use base64::{
    display::Base64Display,
    prelude::BASE64_STANDARD,
    Engine,
};
use ed25519_consensus::{
    Error as Ed25519Error,
    Signature as Ed25519Signature,
    SigningKey as Ed25519SigningKey,
    VerificationKey as Ed25519VerificationKey,
};
use rand::{
    CryptoRng,
    RngCore,
};
use sha2::{
    Digest as _,
    Sha256,
};
use zeroize::{
    Zeroize,
    ZeroizeOnDrop,
};

use crate::primitive::v1::{
    Address,
    AddressError,
    ADDRESS_LEN,
};

/// An Ed25519 signing key.
// *Implementation note*: this is currently a refinement type around
// ed25519_consensus::SigningKey overriding its Debug implementation
// to not accidentally leak it.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SigningKey(Ed25519SigningKey);

impl SigningKey {
    /// Generates a new signing key.
    pub fn new<R: RngCore + CryptoRng>(rng: R) -> Self {
        Self(Ed25519SigningKey::new(rng))
    }

    /// Creates a signature on `msg` using this key.
    #[must_use]
    pub fn sign(&self, msg: &[u8]) -> Signature {
        Signature(self.0.sign(msg))
    }

    /// Returns the byte encoding of the signing key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Returns the byte encoding of the signing key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Returns the verification key associated with this signing key.
    #[must_use]
    pub fn verification_key(&self) -> VerificationKey {
        VerificationKey {
            key: self.0.verification_key(),
            address_bytes: OnceLock::new(),
        }
    }

    /// Returns the address bytes of the verification key associated with this signing key.
    #[must_use]
    pub fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        *self.verification_key().address_bytes()
    }

    /// Attempts to create an Astria bech32m `[Address]` with the given prefix.
    ///
    /// # Errors
    /// Returns an [`AddressError`] if an address could not be constructed
    /// with the given prefix. Usually if the prefix was too long or contained
    /// characters not allowed by bech32m.
    pub fn try_address(&self, prefix: &str) -> Result<Address, AddressError> {
        Address::builder()
            .prefix(prefix)
            .array(self.address_bytes())
            .try_build()
    }
}

impl Debug for SigningKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("SigningKey")
            .field("verification_key", &self.verification_key())
            .finish_non_exhaustive() // avoids printing secret fields
    }
}

impl<'a> From<&'a SigningKey> for VerificationKey {
    fn from(signing_key: &'a SigningKey) -> VerificationKey {
        signing_key.verification_key()
    }
}

impl TryFrom<&[u8]> for SigningKey {
    type Error = ed25519_consensus::Error;

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(Ed25519SigningKey::try_from(slice)?))
    }
}

impl From<[u8; 32]> for SigningKey {
    fn from(seed: [u8; 32]) -> Self {
        Self(Ed25519SigningKey::from(seed))
    }
}

/// An Ed25519 verification key.
#[derive(Clone)]
pub struct VerificationKey {
    key: Ed25519VerificationKey,
    // The address-bytes are lazily-initialized.  Since it may or may not be initialized for any
    // given instance of a verification key, it is excluded from `PartialEq`, `Eq`, `PartialOrd`,
    // `Ord` and `Hash` impls.
    address_bytes: OnceLock<[u8; ADDRESS_LEN]>,
}

impl VerificationKey {
    /// Returns the byte encoding of the verification key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    /// Returns the byte encoding of the verification key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.key.as_bytes()
    }

    /// Verifies `signature` on the given `msg`.
    ///
    /// # Errors
    /// Returns an error if verification fails.
    pub fn verify(&self, signature: &Signature, msg: &[u8]) -> Result<(), Error> {
        self.key.verify(&signature.0, msg).map_err(Error)
    }

    /// Returns the sequencer address of this verification key.
    ///
    /// The address is the first 20 bytes of the sha256 hash of the verification key.
    #[must_use]
    pub fn address_bytes(&self) -> &[u8; ADDRESS_LEN] {
        self.address_bytes.get_or_init(|| {
            fn first_20(array: [u8; 32]) -> [u8; ADDRESS_LEN] {
                [
                    array[0], array[1], array[2], array[3], array[4], array[5], array[6], array[7],
                    array[8], array[9], array[10], array[11], array[12], array[13], array[14],
                    array[15], array[16], array[17], array[18], array[19],
                ]
            }
            /// this ensures that `ADDRESS_LEN` is never accidentally changed to a value
            /// that would violate this assumption.
            #[allow(clippy::assertions_on_constants)]
            const _: () = assert!(ADDRESS_LEN <= 32);
            let bytes: [u8; 32] = Sha256::digest(self).into();
            first_20(bytes)
        })
    }
}

impl Debug for VerificationKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut debug_struct = formatter.debug_struct("VerificationKey");
        debug_struct.field("key", &BASE64_STANDARD.encode(self.key.as_ref()));
        if let Some(address_bytes) = self.address_bytes.get() {
            debug_struct.field("address_bytes", &BASE64_STANDARD.encode(address_bytes));
        } else {
            debug_struct.field("address_bytes", &"unset");
        }
        debug_struct.finish()
    }
}

impl Display for VerificationKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Base64Display::new(self.key.as_ref(), &BASE64_STANDARD).fmt(formatter)
    }
}

impl PartialEq for VerificationKey {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl Eq for VerificationKey {}

impl PartialOrd for VerificationKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VerificationKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl Hash for VerificationKey {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.key.hash(hasher);
    }
}

impl AsRef<[u8]> for VerificationKey {
    fn as_ref(&self) -> &[u8] {
        self.key.as_ref()
    }
}

impl TryFrom<&[u8]> for VerificationKey {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self, Error> {
        let key = Ed25519VerificationKey::try_from(slice).map_err(Error)?;
        Ok(Self {
            key,
            address_bytes: OnceLock::new(),
        })
    }
}

impl TryFrom<[u8; 32]> for VerificationKey {
    type Error = Error;

    fn try_from(bytes: [u8; 32]) -> Result<Self, Self::Error> {
        let key = Ed25519VerificationKey::try_from(bytes).map_err(Error)?;
        Ok(Self {
            key,
            address_bytes: OnceLock::new(),
        })
    }
}

/// An Ed25519 signature.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Signature(Ed25519Signature);

impl Signature {
    /// Returns the bytes of the signature.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }
}

impl Debug for Signature {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        (&self.0 as &dyn Debug).fmt(formatter)
    }
}

impl Display for Signature {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Base64Display::new(&self.0.to_bytes(), &BASE64_STANDARD).fmt(formatter)
    }
}

impl From<[u8; 64]> for Signature {
    fn from(bytes: [u8; 64]) -> Self {
        Self(Ed25519Signature::from(bytes))
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = Error;

    fn try_from(slice: &[u8]) -> Result<Self, Error> {
        let signature = Ed25519Signature::try_from(slice).map_err(Error)?;
        Ok(Self(signature))
    }
}

/// An error related to Ed25519 signing.
#[derive(Copy, Clone, Eq, PartialEq, thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error(Ed25519Error);

#[cfg(test)]
mod tests {
    use super::*;

    // From https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
    #[test]
    // allow: we want explicit assertions here to match the documented expected behavior.
    #[allow(clippy::nonminimal_bool)]
    fn verification_key_comparisons_should_be_consistent() {
        // A key which compares greater than "low" ones below, and with its address uninitialized.
        let high_uninit = VerificationKey {
            key: SigningKey::from([255; 32]).0.verification_key(),
            address_bytes: OnceLock::new(),
        };
        // A key equal to `high_uninit`, but with its address initialized.
        let high_init = VerificationKey {
            key: high_uninit.key,
            address_bytes: OnceLock::new(),
        };
        // A key which compares less than "high" ones above, and with its address uninitialized.
        let low_uninit = VerificationKey {
            key: SigningKey::from([0; 32]).0.verification_key(),
            address_bytes: OnceLock::new(),
        };
        // A key equal to `low_uninit`, but with its address initialized.
        let low_init = VerificationKey {
            key: low_uninit.key,
            address_bytes: OnceLock::new(),
        };

        assert!(high_uninit.cmp(&high_uninit) == Ordering::Equal);
        assert!(high_uninit.cmp(&high_init) == Ordering::Equal);
        assert!(high_init.cmp(&high_uninit) == Ordering::Equal);
        assert!(high_init.cmp(&high_init) == Ordering::Equal);
        assert!(high_uninit.cmp(&low_uninit) == Ordering::Greater);
        assert!(high_uninit.cmp(&low_init) == Ordering::Greater);
        assert!(high_init.cmp(&low_uninit) == Ordering::Greater);
        assert!(high_init.cmp(&low_init) == Ordering::Greater);
        assert!(low_uninit.cmp(&high_uninit) == Ordering::Less);
        assert!(low_uninit.cmp(&high_init) == Ordering::Less);
        assert!(low_init.cmp(&high_uninit) == Ordering::Less);
        assert!(low_init.cmp(&high_init) == Ordering::Less);

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal)
        assert!(high_uninit == high_uninit); // Some(Equal)
        assert!(high_uninit == high_init); // Some(Equal)
        assert!(high_init == high_uninit); // Some(Equal)
        assert!(high_init == high_init); // Some(Equal)
        assert!(!(high_uninit == low_uninit)); // Some(Greater)
        assert!(!(high_uninit == low_init)); // Some(Greater)
        assert!(!(high_init == low_uninit)); // Some(Greater)
        assert!(!(high_init == low_init)); // Some(Greater)
        assert!(!(low_uninit == high_uninit)); // Some(Less)
        assert!(!(low_uninit == high_init)); // Some(Less)
        assert!(!(low_init == high_uninit)); // Some(Less)
        assert!(!(low_init == high_init)); // Some(Less)

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)
        assert!(low_uninit < high_uninit); // Some(Less)
        assert!(low_uninit < high_init); // Some(Less)
        assert!(low_init < high_uninit); // Some(Less)
        assert!(low_init < high_init); // Some(Less)
        assert!(!(high_uninit < high_uninit)); // Some(Equal)
        assert!(!(high_uninit < high_init)); // Some(Equal)
        assert!(!(high_init < high_uninit)); // Some(Equal)
        assert!(!(high_init < high_init)); // Some(Equal)
        assert!(!(high_uninit < low_uninit)); // Some(Greater)
        assert!(!(high_uninit < low_init)); // Some(Greater)
        assert!(!(high_init < low_uninit)); // Some(Greater)
        assert!(!(high_init < low_init)); // Some(Greater)

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert!(high_uninit > low_uninit); // Some(Greater)
        assert!(high_uninit > low_init); // Some(Greater)
        assert!(high_init > low_uninit); // Some(Greater)
        assert!(high_init > low_init); // Some(Greater)
        assert!(!(high_uninit > high_uninit)); // Some(Equal)
        assert!(!(high_uninit > high_init)); // Some(Equal)
        assert!(!(high_init > high_uninit)); // Some(Equal)
        assert!(!(high_init > high_init)); // Some(Equal)
        assert!(!(low_uninit > high_uninit)); // Some(Less)
        assert!(!(low_uninit > high_init)); // Some(Less)
        assert!(!(low_init > high_uninit)); // Some(Less)
        assert!(!(low_init > high_init)); // Some(Less)

        // 4. a <= b if and only if a < b || a == b
        assert!(low_uninit <= high_uninit); // a < b
        assert!(low_uninit <= high_init); // a < b
        assert!(low_init <= high_uninit); // a < b
        assert!(low_init <= high_init); // a < b
        assert!(high_uninit <= high_uninit); // a == b
        assert!(high_uninit <= high_init); // a == b
        assert!(high_init <= high_uninit); // a == b
        assert!(!(high_uninit <= low_uninit)); // a > b
        assert!(!(high_uninit <= low_init)); // a > b
        assert!(!(high_init <= low_uninit)); // a > b
        assert!(!(high_init <= low_init)); // a > b

        // 5. a >= b if and only if a > b || a == b
        assert!(high_uninit >= low_uninit); // a > b
        assert!(high_uninit >= low_init); // a > b
        assert!(high_init >= low_uninit); // a > b
        assert!(high_init >= low_init); // a > b
        assert!(high_uninit >= high_uninit); // a == b
        assert!(high_uninit >= high_init); // a == b
        assert!(high_init >= high_uninit); // a == b
        assert!(high_init >= high_init); // a == b
        assert!(!(low_uninit >= high_uninit)); // a < b
        assert!(!(low_uninit >= high_init)); // a < b
        assert!(!(low_init >= high_uninit)); // a < b
        assert!(!(low_init >= high_init)); // a < b

        // 6. a != b if and only if !(a == b)
        assert!(high_uninit != low_uninit); // asserted !(high == low) above
        assert!(high_uninit != low_init); // asserted !(high == low) above
        assert!(high_init != low_uninit); // asserted !(high == low) above
        assert!(high_init != low_init); // asserted !(high == low) above
        assert!(low_uninit != high_uninit); // asserted !(low == high) above
        assert!(low_uninit != high_init); // asserted !(low == high) above
        assert!(low_init != high_uninit); // asserted !(low == high) above
        assert!(low_init != high_init); // asserted !(low == high) above
        assert!(!(high_uninit != high_uninit)); // asserted high == high above
        assert!(!(high_uninit != high_init)); // asserted high == high above
        assert!(!(high_init != high_uninit)); // asserted high == high above
        assert!(!(high_init != high_init)); // asserted high == high above
    }

    #[test]
    // From https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq
    fn verification_key_hash_and_eq_should_be_consistent() {
        // Check verification keys compare equal if and only if their keys are equal.
        let key0 = VerificationKey {
            key: SigningKey::from([0; 32]).0.verification_key(),
            address_bytes: OnceLock::new(),
        };
        let other_key0 = VerificationKey {
            key: SigningKey::from([0; 32]).0.verification_key(),
            address_bytes: OnceLock::new(),
        };
        let key1 = VerificationKey {
            key: SigningKey::from([1; 32]).0.verification_key(),
            address_bytes: OnceLock::new(),
        };

        assert!(key0 == other_key0);
        assert!(key0 != key1);

        // Check verification keys' std hashes compare equal if and only if their keys are equal.
        let std_hash = |verification_key: &VerificationKey| -> u64 {
            let mut hasher = std::hash::DefaultHasher::new();
            verification_key.hash(&mut hasher);
            hasher.finish()
        };
        assert!(std_hash(&key0) == std_hash(&other_key0));
        assert!(std_hash(&key0) != std_hash(&key1));
    }
}
