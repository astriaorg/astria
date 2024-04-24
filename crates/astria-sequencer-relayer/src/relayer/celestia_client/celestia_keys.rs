use std::{
    fmt::{
        self,
        Debug,
        Formatter,
    },
    fs,
    path::Path,
};

use k256::ecdsa::{
    signature::Signer,
    Signature,
};
use tendermint::{
    account::Id as AccountId,
    private_key::Secp256k1 as SigningKey,
    public_key::Secp256k1 as VerificationKey,
};
use thiserror::Error;
use tracing::instrument;

/// Errors which can be returned when parsing a Celestia signing key file.
///
/// Note that the path to the file is not included in any of the variants in order to avoid
/// displaying potentially sensitive path information in logs.
#[derive(Error, Debug)]
#[non_exhaustive]
pub(crate) enum Error {
    /// Failed to read the given file.
    #[error("failed to read celestia signing key file")]
    ReadFile(#[from] std::io::Error),
    /// Failed to decode the file contents from hex.
    #[error("failed to hex-decode celestia signing key")]
    DecodeFromHex(#[from] FromHexError),
    /// The file doesn't contain a valid signing key.
    #[error("invalid signing key")]
    InvalidSigningKey,
}

/// An error while decoding a hex string.
#[derive(Error, Debug)]
#[error(transparent)]
pub(crate) struct FromHexError(#[from] hex::FromHexError);

#[derive(Clone)]
pub(crate) struct CelestiaKeys {
    /// The celestia account address; defined as SHA256(public key)[..20].
    pub(crate) address: AccountId,

    /// The signing (secret) key.
    pub(crate) signing_key: SigningKey,

    /// The verifying (public) key.
    pub(crate) verification_key: VerificationKey,
}

impl CelestiaKeys {
    /// Constructs `CelestiaKeys` from the given file.
    ///
    /// The file should be a hex-encoded secp256k1 secret key, such as could be output via
    /// `celestia-appd keys export <keyname> --keyring-backend=... --home=... --unsafe
    /// --unarmored-hex`
    #[instrument(skip_all, fields(path = %path.as_ref().display(), err))]
    pub(crate) fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let hex_encoded = fs::read_to_string(path)?;
        let bytes = hex::decode(hex_encoded.trim())
            .map_err(|error| Error::DecodeFromHex(FromHexError(error)))?;
        let key = SigningKey::from_slice(&bytes).map_err(|_| Error::InvalidSigningKey)?;
        Ok(Self::from(key))
    }

    pub(crate) fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }
}

impl From<SigningKey> for CelestiaKeys {
    fn from(signing_key: SigningKey) -> Self {
        let verification_key = *signing_key.verifying_key();
        let address = AccountId::from(verification_key);
        Self {
            address,
            signing_key,
            verification_key,
        }
    }
}

impl Debug for CelestiaKeys {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CelestiaKeys")
            .field("address", &self.address)
            .field("signing_key", &"...")
            .field("verification_key", &self.verification_key)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_construct_from_file() {
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(
            tmp_file.path(),
            b"c8076374e2a4a58db1c924e3dafc055e9685481054fe99e58ed67f5c6ed80e62",
        )
        .unwrap();
        CelestiaKeys::from_path(tmp_file.path()).unwrap();

        // Check leading and trailing whitespace are ignored.
        fs::write(
            tmp_file.path(),
            b"\nc8076374e2a4a58db1c924e3dafc055e9685481054fe99e58ed67f5c6ed80e62 \n",
        )
        .unwrap();
        CelestiaKeys::from_path(tmp_file.path()).unwrap();
    }

    #[test]
    fn should_fail_to_construct_from_missing_file() {
        let error = CelestiaKeys::from_path("missing").unwrap_err();
        assert!(matches!(error, Error::ReadFile(_)));
    }

    #[test]
    fn should_fail_to_construct_from_file_with_non_hex() {
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp_file.path(), b"not hex").unwrap();
        let error = CelestiaKeys::from_path(tmp_file.path()).unwrap_err();
        assert!(matches!(error, Error::DecodeFromHex(_)));
    }

    #[test]
    fn should_fail_to_construct_from_file_with_invalid_key() {
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp_file.path(), b"abcdef").unwrap();
        let error = CelestiaKeys::from_path(tmp_file.path()).unwrap_err();
        assert!(matches!(error, Error::InvalidSigningKey));
    }
}
