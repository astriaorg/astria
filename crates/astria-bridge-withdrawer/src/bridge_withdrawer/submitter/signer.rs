use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
};
use astria_eyre::eyre::{
    self,
    bail,
    eyre,
    Context,
};

pub(crate) struct SequencerKey {
    address: Address,
    signing_key: SigningKey,
}

pub(crate) struct SequencerKeyBuilder {
    path: Option<PathBuf>,
    prefix: Option<String>,
}

impl SequencerKeyBuilder {
    /// Sets the path from which the sequencey key is read.
    ///
    /// The file at `path` should contain a hex-encoded ed25519 secret key.
    pub(crate) fn path<P: AsRef<Path>>(self, path: P) -> Self {
        Self {
            path: Some(path.as_ref().to_path_buf()),
            ..self
        }
    }

    /// Sets the prefix for constructing a bech32m sequencer address.
    ///
    /// The prefix must be a valid bech32 human-readable-prefix (Hrp).
    pub(crate) fn prefix<S: AsRef<str>>(self, prefix: S) -> Self {
        Self {
            prefix: Some(prefix.as_ref().to_string()),
            ..self
        }
    }

    pub(crate) fn try_build(self) -> eyre::Result<SequencerKey> {
        let Some(path) = self.path else {
            bail!("path to sequencer key file must be set");
        };
        let Some(prefix) = self.prefix else {
            bail!(
                "a prefix to construct bech32m complicant astria addresses from the signing key \
                 must be set"
            );
        };
        let hex = fs::read_to_string(&path).wrap_err_with(|| {
            format!("failed to read sequencer key from path: {}", path.display())
        })?;
        let bytes: [u8; 32] = hex::decode(hex.trim())
            .wrap_err_with(|| format!("failed to decode hex: {}", path.display()))?
            .try_into()
            .map_err(|_| {
                eyre!(
                    "invalid private key length; must be 32 bytes: {}",
                    path.display()
                )
            })?;
        let signing_key = SigningKey::from(bytes);
        let address = Address::builder()
            .array(signing_key.address_bytes())
            .prefix(&prefix)
            .try_build()
            .wrap_err_with(|| {
                format!(
                    "failed constructing valid sequencer address using the provided prefix \
                     `{prefix}`"
                )
            })?;

        Ok(SequencerKey {
            address,
            signing_key,
        })
    }
}

impl SequencerKey {
    pub(crate) fn builder() -> SequencerKeyBuilder {
        SequencerKeyBuilder {
            path: None,
            prefix: None,
        }
    }

    pub(crate) fn address(&self) -> &Address {
        &self.address
    }

    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
}
