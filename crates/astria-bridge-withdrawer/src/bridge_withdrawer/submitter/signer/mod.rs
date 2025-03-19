mod frost;
mod sequencer_key;

use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};

pub(crate) enum Signer {
    Single(Box<sequencer_key::SequencerKey>),
    Threshold(frost::FrostSigner),
}

impl Signer {
    pub(crate) fn address(&self) -> &astria_core::primitive::v1::Address {
        match self {
            Self::Single(signer) => signer.address(),
            Self::Threshold(signer) => signer.address(),
        }
    }

    pub(crate) async fn sign(
        &self,
        tx: astria_core::protocol::transaction::v1::TransactionBody,
    ) -> eyre::Result<astria_core::protocol::transaction::v1::Transaction> {
        match self {
            Self::Single(signer) => Ok(signer.sign(tx)),
            Self::Threshold(signer) => signer.sign(tx).await,
        }
    }
}

pub(super) struct Builder {
    pub(super) no_frost_threshold_signing: bool,
    pub(super) frost_min_signers: usize,
    pub(super) frost_public_key_package_path: String,
    pub(super) frost_participant_endpoints: String,
    pub(super) sequencer_key_path: String,
    pub(super) sequencer_address_prefix: String,
}

impl Builder {
    pub(super) fn try_build(self) -> eyre::Result<Signer> {
        let Self {
            no_frost_threshold_signing,
            frost_min_signers,
            frost_public_key_package_path,
            frost_participant_endpoints,
            sequencer_key_path,
            sequencer_address_prefix,
        } = self;
        let signer = if no_frost_threshold_signing {
            Signer::Single(Box::new(
                sequencer_key::SequencerKey::builder()
                    .path(&sequencer_key_path)
                    .prefix(sequencer_address_prefix)
                    .try_build()
                    .wrap_err_with(|| {
                        format!(
                            "failed to load sequencer private key from path `{sequencer_key_path}`"
                        )
                    })?,
            ))
        } else {
            Signer::Threshold(
                frost::Builder {
                    frost_min_signers,
                    frost_participant_endpoints,
                    frost_public_key_package_path,
                    sequencer_address_prefix,
                }
                .try_build()
                .wrap_err("failed to initialize frost signer")?,
            )
        };
        Ok(signer)
    }
}
