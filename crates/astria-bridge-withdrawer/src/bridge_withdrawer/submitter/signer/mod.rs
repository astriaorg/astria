mod frost_signer;
mod sequencer_key;

use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
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

pub(crate) async fn make_signer(
    no_frost_threshold_signing: bool,
    frost_min_signers: usize,
    frost_public_key_package_path: String,
    frost_participant_endpoints: Vec<String>,
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
        let public_key_package =
            read_frost_key(&frost_public_key_package_path).wrap_err_with(|| {
                format!(
                    "failed reading frost public key package from file \
                     `{frost_public_key_package_path}`"
                )
            })?;

        let participant_clients = frost_signer::initialize_frost_participant_clients(
            frost_participant_endpoints,
            &public_key_package,
        )
        .await
        .wrap_err("failed to initialize frost participant clients")?;
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

fn read_frost_key<P: AsRef<std::path::Path>>(
    path: P,
) -> eyre::Result<frost_ed25519::keys::PublicKeyPackage> {
    let key_str =
        std::fs::read_to_string(path).wrap_err("failed to read frost public key package")?;
    serde_json::from_str::<frost_ed25519::keys::PublicKeyPackage>(&key_str)
        .wrap_err("failed to deserialize public key package")
}
