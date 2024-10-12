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
    /// perform part 1 of the signing protocol.
    ///
    /// generates participant commitments (used in `prepare-message`)
    /// and nonces (used in part 2).
    Part1(Part1),

    /// generate a signing package given a message to be signed
    /// and commitments from part 1.
    ///
    /// can be executed by a coordinator, or by every individual participant, assuming
    /// all participants use the same commitments generated in part 1.
    PrepareMessage(PrepareMessage),

    /// perform part 2 of the signing protocol.
    ///
    /// generates a signature share using the nonces from part 1 and the
    /// signing package from `prepare-message`.
    Part2(Part2),

    /// aggregate signature shares from part 2 to produce the final signature.
    Aggregate(Aggregate),
}

#[derive(Debug, clap::Args)]
struct Part1 {
    /// path to a file with the secret key package from keygen ceremony
    #[arg(long)]
    secret_key_package_path: String,

    /// path to a file to output the nonces
    #[arg(long)]
    nonces_path: String,
}

impl Part1 {
    async fn run(self) -> eyre::Result<()> {
        let mut rng = thread_rng();

        let Self {
            secret_key_package_path,
            nonces_path,
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

        println!("Writing nonces to {}", nonces_path);
        std::fs::write(nonces_path, hex::encode(nonces.serialize()?).as_bytes())?;

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

    /// path to the signing package output file
    #[arg(long)]
    signing_package_path: String,
}

impl PrepareMessage {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            message,
            signing_package_path,
        } = self;

        let mut commitments: BTreeMap<Identifier, frost_ed25519::round1::SigningCommitments> =
            BTreeMap::new();

        loop {
            println!("Enter commitment for a participant (or 'done' to finish)",);
            let input = read_line_raw().await?;
            if input == "done" {
                break;
            }
            let Ok(commitments_with_id) = serde_json::from_str::<CommitmentsWithIdentifier>(&input)
            else {
                continue;
            };
            commitments.insert(
                commitments_with_id.identifier,
                commitments_with_id.commitments,
            );
            println!("Received {} commitments", commitments.len());
        }

        let signing_package = frost_ed25519::SigningPackage::new(commitments, message.as_bytes());
        println!("Writing signing package to {}", signing_package_path);
        std::fs::write(
            signing_package_path,
            hex::encode(signing_package.serialize()?).as_bytes(),
        )?;
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Part2 {
    /// path to a file with the secret key package from keygen ceremony
    #[arg(long)]
    secret_key_package_path: String,

    /// path to nonces file from part 1
    #[arg(long)]
    nonces_path: String,

    /// path to the signing package
    #[arg(long)]
    signing_package_path: String,
}

impl Part2 {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            secret_key_package_path,
            nonces_path,
            signing_package_path,
        } = self;

        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )?;
        let nonces_str =
            std::fs::read_to_string(&nonces_path).wrap_err("failed to read nonces file")?;
        let nonces = frost_ed25519::round1::SigningNonces::deserialize(
            &hex::decode(nonces_str).wrap_err("failed to decode nonces")?,
        )?;
        let signing_package_str = std::fs::read_to_string(&signing_package_path)
            .wrap_err("failed to read signing package file")?;
        let signing_package = frost_ed25519::SigningPackage::deserialize(
            &hex::decode(signing_package_str).wrap_err("failed to decode signing package")?,
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
    /// path to the signing package
    #[arg(long)]
    signing_package_path: String,

    /// path to a file with the public key package from keygen ceremony
    #[arg(long)]
    public_key_package_path: String,
}

impl Aggregate {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            signing_package_path,
            public_key_package_path,
        } = self;

        let mut sig_shares: BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> =
            BTreeMap::new();
        loop {
            println!("Enter signature share for a participant (or 'done' to finish)");
            let input = read_line_raw().await?;
            if input == "done" {
                break;
            }
            let Ok(sig_share) = serde_json::from_str::<SignatureShareWithIdentifier>(&input) else {
                continue;
            };
            sig_shares.insert(sig_share.identifier, sig_share.signature_share);
            println!("Received {} signature shares", sig_shares.len());
        }

        let signing_package_str = std::fs::read_to_string(&signing_package_path)
            .wrap_err("failed to read signing package from file")?;
        let signing_package = frost_ed25519::SigningPackage::deserialize(
            &hex::decode(signing_package_str).wrap_err("failed to decode signing package")?,
        )?;

        let public_key_package_file =
            std::fs::read_to_string(&public_key_package_path).wrap_err(format!(
                "failed to read public key package from file: {}",
                public_key_package_path
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
