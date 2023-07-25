use std::path::Path;

use ed25519_consensus::{
    SigningKey,
    VerificationKey,
};
use eyre::{
    bail,
    WrapErr as _,
};
use tendermint::account;
use tendermint_config::PrivValidatorKey;
use tracing::instrument;
use zeroize::{
    Zeroize,
    ZeroizeOnDrop,
};

/// `Validator` holds the ed25519 keys to sign and verify tendermint
/// messages. It also contains its address (`AccountId`) in the tendermint network.
#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct Validator {
    /// The tendermint validator account address; defined as
    /// Sha256(verification_key)[..20].
    #[zeroize(skip)]
    pub(crate) address: account::Id,

    /// The ed25519 signing key of this validator.
    pub(crate) signing_key: SigningKey,

    #[zeroize(skip)]
    /// The ed25519 verification key of this validator.
    pub(crate) verification_key: VerificationKey,
}

impl Validator {
    pub fn address(&self) -> &account::Id {
        &self.address
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn verification_key(&self) -> &VerificationKey {
        &self.verification_key
    }

    /// Constructs a `Validator` from a json formatted tendermint private validator key.
    ///
    /// This file is frequently called `private_validator_key.json` and is generated during
    /// the initialization of a tendermint node.
    #[instrument(skip_all, fields(path = %path.as_ref().display(), err))]
    pub fn from_path(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let key = PrivValidatorKey::load_json_file(&path.as_ref())
            .wrap_err("failed reading private validator key from file")?;
        Self::from_priv_validator_key(key)
    }

    pub fn from_priv_validator_key(key: PrivValidatorKey) -> eyre::Result<Self> {
        let PrivValidatorKey {
            address,
            pub_key,
            priv_key,
        } = key;
        let Some(tendermint_signing_key) = priv_key.ed25519_signing_key().cloned() else {
            bail!("deserialized private key was not ed25519");
        };
        let signing_key = tendermint_signing_key.try_into().wrap_err(
            "failed constructing ed25519 signing key from deserialized tendermint private key",
        )?;
        let Some(tendermint_verification_key) = pub_key.ed25519() else {
            bail!("deserialized public key was not ed25519");
        };
        let verification_key = tendermint_verification_key.try_into().wrap_err(
            "failed constructing ed25519 verification key from deserialized tendermint public key",
        )?;

        Ok(Self {
            address,
            signing_key,
            verification_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Validator;

    const VALIDATOR_FILE_BODY: &str = r#"
{
  "address": "468646B2BD3E75229B2163F4D7905748FEC7603E",
  "pub_key": {
    "type": "tendermint/PubKeyEd25519",
    "value": "Fj/2NzG404f+CjHJUThMXNS7xJY5GMPuFVlKMKb86MA="
  },
  "priv_key": {
    "type": "tendermint/PrivKeyEd25519",
    "value": "1hBYYTBKxkMODNTW6Pk//kA023UAkpgSLhM0SjwndSkWP/Y3MbjTh/4KMclROExc1LvEljkYw+4VWUowpvzowA=="
  }
}
"#;

    #[test]
    fn valid_validator_keys_can_be_read_from_file() {
        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp_file.path(), VALIDATOR_FILE_BODY).unwrap();
        Validator::from_path(tmp_file.path()).unwrap();
    }
}
