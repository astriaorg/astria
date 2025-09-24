use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};

mod frost;
mod key;

pub(crate) struct Signer {
    inner: SignerInner,
}

impl From<key::Key> for Signer {
    fn from(value: key::Key) -> Self {
        Self {
            inner: SignerInner::Key(Box::new(value)),
        }
    }
}

enum SignerInner {
    Key(Box<key::Key>),
    Frost(Box<frost::Frost>),
}

impl Signer {
    pub(crate) fn address(&self) -> &astria_core::primitive::v1::Address {
        match &self.inner {
            SignerInner::Key(signer) => signer.address(),
            SignerInner::Frost(signer) => signer.address(),
        }
    }

    pub(crate) async fn initialize(&mut self) -> eyre::Result<()> {
        if let SignerInner::Frost(frost_signer) = &mut self.inner {
            frost_signer
                .initialize_participant_clients()
                .await
                .wrap_err(
                    "failed initializing clients participating in frost threshold signing scheme",
                )?;
        }
        Ok(())
    }

    pub(crate) async fn sign(
        &self,
        tx: astria_core::protocol::transaction::v1::TransactionBody,
    ) -> eyre::Result<astria_core::protocol::transaction::v1::Transaction> {
        match &self.inner {
            SignerInner::Key(signer) => Ok(signer.sign(tx)),
            SignerInner::Frost(signer) => signer.sign(tx).await,
        }
    }
}

pub(super) struct Builder {
    pub(super) no_frost_threshold_signing: bool,
    pub(super) frost_min_signers: usize,
    pub(super) frost_public_key_package_path: String,
    pub(super) frost_participant_endpoints: crate::config::FrostParticipantEndpoints,
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
        let inner = if no_frost_threshold_signing {
            SignerInner::Key(Box::new(
                key::Key::builder()
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
            SignerInner::Frost(Box::new(
                frost::Builder {
                    frost_min_signers,
                    frost_participant_endpoints,
                    frost_public_key_package_path,
                    sequencer_address_prefix,
                }
                .try_build()
                .wrap_err("failed to construct frost threshold signer")?,
            ))
        };
        Ok(Signer {
            inner,
        })
    }
}
