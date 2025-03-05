mod frost_signer;
mod sequencer_key;

use std::collections::HashMap;

use astria_core::generated::astria::signer::v1::frost_participant_service_client::FrostParticipantServiceClient;
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use frost_ed25519::{
    keys::PublicKeyPackage,
    Identifier,
};

pub(crate) enum Signer {
    Single(Box<sequencer_key::SequencerKey>),
    Threshold(frost_signer::FrostSigner),
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

pub(crate) fn make_signer(
    no_frost_threshold_signing: bool,
    frost_min_signers: usize,
    public_key_package: Option<PublicKeyPackage>,
    frost_participant_clients: Option<
        HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>,
    >,
    sequencer_key_path: String,
    sequencer_address_prefix: String,
) -> eyre::Result<Signer> {
    let signer = if no_frost_threshold_signing {
        Signer::Single(Box::new(
            sequencer_key::SequencerKey::builder()
                .path(sequencer_key_path)
                .prefix(sequencer_address_prefix)
                .try_build()
                .wrap_err("failed to load sequencer private key")?,
        ))
    } else {
        let participant_clients = frost_participant_clients.ok_or(eyre::eyre!(
            "frost participant clients must be set when using frost threshold signing"
        ))?;
        let public_key_package = public_key_package.ok_or(eyre::eyre!(
            "frost public key package must be set when using frost threshold signing"
        ))?;
        Signer::Threshold(
            frost_signer::FrostSignerBuilder::new()
                .min_signers(frost_min_signers)
                .public_key_package(public_key_package)
                .participant_clients(participant_clients)
                .address_prefix(sequencer_address_prefix)
                .try_build()
                .wrap_err("failed to initialize frost signer")?,
        )
    };
    Ok(signer)
}
