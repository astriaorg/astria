use std::fmt::{
    self,
    Debug,
    Formatter,
};

use ed25519_consensus::{
    Signature,
    SigningKey as Ed25519SigningKey,
    VerificationKey,
};
use rand::{
    CryptoRng,
    RngCore,
};
use zeroize::{
    Zeroize,
    ZeroizeOnDrop,
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
        self.0.sign(msg)
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
        self.0.verification_key()
    }
}

impl Debug for SigningKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("SigningKey")
            .field("verification_key", &self.0.verification_key())
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
