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
    WrapErr as _,
};

pub(super) struct SequencerKey {
    pub(super) address: Address,
    pub(super) signing_key: SigningKey,
}

impl SequencerKey {
    /// Construct a `SequencerKey` from a file.
    ///
    /// The file should contain a hex-encoded ed25519 secret key.
    pub(super) fn try_from_path<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let hex = fs::read_to_string(path)?;
        let bytes: [u8; 32] = hex::decode(hex.trim())?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
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
