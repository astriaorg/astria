use std::path::Path;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tendermint::account;
use tendermint_config::PrivValidatorKey;
use tracing::instrument;

/// `Validator` holds the ed25519 keys to sign and verify tendermint
/// messages. It also contains its address (`AccountId`) in the tendermint network.
#[derive(Clone, Debug)]
pub(crate) struct Validator {
    /// The tendermint validator account address; defined as
    /// Sha256(verification_key)[..20].
    pub(super) address: account::Id,
}

impl Validator {
    /// Constructs a `Validator` from a json formatted tendermint private validator key.
    ///
    /// This file is frequently called `private_validator_key.json` and is generated during
    /// the initialization of a tendermint node.
    #[instrument(skip_all, fields(path = %path.as_ref().display(), err))]
    pub(crate) fn from_path(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let key = PrivValidatorKey::load_json_file(&path.as_ref())
            .wrap_err("failed reading private validator key from file")?;
        Ok(Self {
            address: key.address,
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
