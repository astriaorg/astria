use std::{
    fs,
    path::Path,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        Address,
        ASTRIA_ADDRESS_PREFIX,
    },
};
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
};

pub(crate) struct SequencerKey {
    pub(crate) address: Address,
    pub(crate) signing_key: SigningKey,
}

impl SequencerKey {
    /// Construct a `SequencerKey` from a file.
    ///
    /// The file should contain a hex-encoded ed25519 secret key.
    pub(crate) fn try_from_path<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let hex = fs::read_to_string(path.as_ref()).wrap_err_with(|| {
            format!(
                "failed to read sequencer key from path: {}",
                path.as_ref().display()
            )
        })?;
        let bytes: [u8; 32] = hex::decode(hex.trim())
            .wrap_err_with(|| format!("failed to decode hex: {}", path.as_ref().display()))?
            .try_into()
            .map_err(|_| {
                eyre!(
                    "invalid private key length; must be 32 bytes: {}",
                    path.as_ref().display()
                )
            })?;
        let signing_key = SigningKey::from(bytes);

        Ok(Self {
            address: Address::builder()
                .array(signing_key.verification_key().address_bytes())
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .wrap_err("failed to construct Sequencer address")?,
            signing_key,
        })
    }
}
