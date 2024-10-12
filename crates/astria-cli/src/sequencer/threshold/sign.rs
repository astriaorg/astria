use std::collections::BTreeMap;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::{
    self,
    Identifier,
};
use rand::thread_rng;
use serde::{
    Deserialize,
    Serialize,
};

use super::read_line_raw;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::PrepareMessage(prepare_message) => prepare_message.run().await,
            SubCommand::Part1(part1) => part1.run().await,
            SubCommand::Part2(part2) => part2.run().await,
            SubCommand::Aggregate(aggregate) => aggregate.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    Part1(Part1),

    // executed by the coordinator, or by every individual participant, assuming
    // all participants use the same commitments generated in part1
    PrepareMessage(PrepareMessage),

    Part2(Part2),

    Aggregate(Aggregate),
}

#[derive(Debug, clap::Args)]
struct Part1 {
    /// path to a file with the secret key package from keygen ceremony
    #[arg(long)]
    secret_key_package_path: String,
}

impl Part1 {
    async fn run(self) -> eyre::Result<()> {
        let mut rng = thread_rng();

        let Self {
            secret_key_package_path,
        } = self;

        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )?;
        let (nonces, commitments) =
            frost_ed25519::round1::commit(secret_package.signing_share(), &mut rng);
        let commitments_with_id = CommitmentsWithIdentifier {
            identifier: *secret_package.identifier(),
            commitments,
        };

        println!("Our nonces are: {}", hex::encode(nonces.serialize()?));
        println!(
            "Our commitments are: {}",
            serde_json::to_string(&commitments_with_id)?
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CommitmentsWithIdentifier {
    identifier: Identifier,
    commitments: frost_ed25519::round1::SigningCommitments,
}

#[derive(Debug, clap::Args)]
struct PrepareMessage {
    /// message to be signed
    #[arg(long)]
    message: String,
}

impl PrepareMessage {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            message,
        } = self;

        let mut commitments: BTreeMap<Identifier, frost_ed25519::round1::SigningCommitments> =
            BTreeMap::new();

        loop {
            println!("Enter commitment for a participant (or 'done' to finish)",);
            let input = read_line_raw().await?;
            if input == "done" {
                break;
            }
            let commitments_with_id = serde_json::from_str::<CommitmentsWithIdentifier>(&input)?;
            commitments.insert(
                commitments_with_id.identifier,
                commitments_with_id.commitments,
            );
        }

        let signing_package = frost_ed25519::SigningPackage::new(commitments, message.as_bytes());
        println!(
            "Signing package: {}",
            hex::encode(signing_package.serialize()?)
        );
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Part2 {
    /// path to a file with the secret key package from keygen ceremony
    #[arg(long)]
    secret_key_package_path: String,

    /// our hex-encoded nonces from part1
    #[arg(long)]
    nonces: String,

    /// hex-encoded signing package
    #[arg(long)]
    signing_package: String,
}

impl Part2 {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            secret_key_package_path,
            nonces,
            signing_package,
        } = self;

        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )?;
        let nonces = frost_ed25519::round1::SigningNonces::deserialize(
            &hex::decode(nonces).wrap_err("failed to decode nonces")?,
        )?;
        let signing_package = frost_ed25519::SigningPackage::deserialize(
            &hex::decode(signing_package).wrap_err("failed to decode signing package")?,
        )?;
        let sig_share = frost_ed25519::round2::sign(&signing_package, &nonces, &secret_package)
            .wrap_err("failed to sign")?;

        println!(
            "Our signature share is: {}",
            serde_json::to_string(&SignatureShareWithIdentifier {
                identifier: *secret_package.identifier(),
                signature_share: sig_share,
            })?
        );

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SignatureShareWithIdentifier {
    identifier: Identifier,
    signature_share: frost_ed25519::round2::SignatureShare,
}

#[derive(Debug, clap::Args)]
struct Aggregate {
    /// hex-encoded signing package
    #[arg(long)]
    signing_package: String,

    /// path to a file with the public key package from keygen ceremony
    #[arg(long)]
    public_key_package_path: String,
}

impl Aggregate {
    async fn run(self) -> eyre::Result<()> {
        let mut sig_shares: BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> =
            BTreeMap::new();
        loop {
            println!("Enter signature share for a participant (or 'done' to finish)");
            let input = read_line_raw().await?;
            if input == "done" {
                break;
            }
            let sig_share = serde_json::from_str::<SignatureShareWithIdentifier>(&input)?;
            sig_shares.insert(sig_share.identifier, sig_share.signature_share);
        }

        let signing_package = frost_ed25519::SigningPackage::deserialize(
            &hex::decode(self.signing_package).wrap_err("failed to decode signing package")?,
        )?;

        let public_key_package_file = std::fs::read_to_string(&self.public_key_package_path)
            .wrap_err(format!(
                "failed to read public key package from file: {}",
                self.public_key_package_path
            ))?;
        let public_key_package = serde_json::from_str::<frost_ed25519::keys::PublicKeyPackage>(
            &public_key_package_file,
        )?;

        let signature =
            frost_ed25519::aggregate(&signing_package, &sig_shares, &public_key_package)
                .wrap_err("failed to aggregate")?;
        println!(
            "Aggregated signature: {}",
            hex::encode(signature.serialize())
        );
        Ok(())
    }
}
