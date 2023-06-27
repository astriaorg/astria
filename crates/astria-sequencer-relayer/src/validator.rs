use std::path::Path;

use eyre::{
    bail,
    WrapErr as _,
};
use subtle_encoding::bech32;
use tendermint::account;
use tracing::instrument;
use zeroize::{
    Zeroize,
    ZeroizeOnDrop,
};

const BECH32_HUMAN_READABLE_PREFIX: &str = "metrovalcons";

/// `Validator` holds the ed25519 keys to sign and verify tendermint
/// messages. It also contains its address in the tendermint network
/// and a bech32 encoded address that is used on the metro network.
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub(crate) struct Validator {
    /// The tendermint validator account address; defined as
    /// Sha256(verification_key)[..20].
    #[zeroize(skip)]
    pub(crate) address: account::Id,
    /// The bech32-encoded validator account address with "metrovalcons" as
    /// as the human readable prefix.
    ///
    /// FIXME: Add a note on what this is used for.
    /// NOTE 1: why apply bech32 to the validator *address* instead of its
    ///         verification (i.e. public) key? tendermint provides `PublicKey::to_bech32`
    /// NOTE 2: tendermint's `PublicKey::to_bech32` prepends the pubkey with extra bytes
    ///         to make it amino compatible. Do we need this too?
    #[zeroize(skip)]
    pub(crate) bech32_address: String,
    /// The ed25519 signing key of this validator.
    pub(crate) signing_key: ed25519_consensus::SigningKey,
    #[zeroize(skip)]
    /// The ed25519 verification key of this validator.
    pub(crate) verification_key: ed25519_consensus::VerificationKey,
}

impl Validator {
    /// Constructs a `Validator` from a json formatted tendermint private validator key.
    ///
    /// This file is frequently called `private_validator_key.json` and is generated during
    /// the initialization of a tendermint node.
    #[instrument(skip_all, fields(path = %path.as_ref().display(), err))]
    pub(crate) fn from_path(path: impl AsRef<Path>) -> eyre::Result<Self> {
        use tendermint_config::PrivValidatorKey;
        let PrivValidatorKey {
            address,
            pub_key,
            priv_key,
        } = PrivValidatorKey::load_json_file(&path.as_ref())
            .wrap_err("failed reading private validator key from file")?;
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
        let bech32_address = bech32::encode(BECH32_HUMAN_READABLE_PREFIX, address);

        Ok(Self {
            address,
            bech32_address,
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
