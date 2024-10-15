use std::collections::BTreeMap;

use astria_core::generated::protocol::transactions::v1alpha1::SignedTransaction;
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
use termion::color;

use super::read_line_raw;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Part1(part1) => part1.run(),
            SubCommand::PrepareMessage(prepare_message) => prepare_message.run().await,
            SubCommand::Part2(part2) => part2.run(),
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
    fn run(self) -> eyre::Result<()> {
        let mut rng = thread_rng();

        let Self {
            secret_key_package_path,
            nonces_path,
        } = self;

        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )
        .wrap_err("failed to deserialize secret key package")?;
        let (nonces, commitments) =
            frost_ed25519::round1::commit(secret_package.signing_share(), &mut rng);
        let commitments_with_id = CommitmentsWithIdentifier {
            identifier: *secret_package.identifier(),
            commitments,
        };

        println!("Writing nonces to {nonces_path}");
        std::fs::write(
            nonces_path,
            hex::encode(nonces.serialize().wrap_err("failed to serialized nonces")?).as_bytes(),
        )
        .wrap_err("failed to write nonces to file")?;

        println!("Our commitments are:",);
        print!("{}", color::Fg(color::Green));
        println!(
            "{}",
            serde_json::to_string(&commitments_with_id)
                .wrap_err("failed to serialize commitments")?
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
    /// path to file with message bytes to be signed
    #[arg(long)]
    message_path: String,

    /// path to the signing package output file
    #[arg(long)]
    signing_package_path: String,
}

impl PrepareMessage {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            message_path,
            signing_package_path,
        } = self;

        let message = std::fs::read(&message_path).wrap_err("failed to read message file")?;

        let mut commitments: BTreeMap<Identifier, frost_ed25519::round1::SigningCommitments> =
            BTreeMap::new();

        loop {
            println!("Enter commitment for a participant (or 'done' to finish)",);
            let input = read_line_raw().await.wrap_err("failed to read line")?;
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

        let signing_package = frost_ed25519::SigningPackage::new(commitments, &message);
        println!("Writing signing package to {signing_package_path}");
        std::fs::write(
            signing_package_path,
            hex::encode(
                signing_package
                    .serialize()
                    .wrap_err("failed to serialize signing package")?,
            )
            .as_bytes(),
        )
        .wrap_err("failed to write signing package to file")?;
        Ok(())
    }
}

// it's okay for all the args to end in `_path`
#[allow(clippy::struct_field_names)]
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
    fn run(self) -> eyre::Result<()> {
        let Self {
            secret_key_package_path,
            nonces_path,
            signing_package_path,
        } = self;

        let secret_package = serde_json::from_slice::<frost_ed25519::keys::KeyPackage>(
            &std::fs::read(secret_key_package_path)
                .wrap_err("failed to read secret key package file")?,
        )
        .wrap_err("failed to deserialize secret key package")?;
        let nonces_str =
            std::fs::read_to_string(&nonces_path).wrap_err("failed to read nonces file")?;
        let nonces = frost_ed25519::round1::SigningNonces::deserialize(
            &hex::decode(nonces_str).wrap_err("failed to decode nonces")?,
        )
        .wrap_err("failed to deserialize nonces")?;
        let signing_package_str = std::fs::read_to_string(&signing_package_path)
            .wrap_err("failed to read signing package file")?;
        let signing_package = frost_ed25519::SigningPackage::deserialize(
            &hex::decode(signing_package_str).wrap_err("failed to decode signing package")?,
        )
        .wrap_err("failed to deserialize signing package")?;
        let sig_share = frost_ed25519::round2::sign(&signing_package, &nonces, &secret_package)
            .wrap_err("failed to sign")?;

        println!("Our signature share is:",);
        print!("{}", color::Fg(color::Green));
        println!(
            "{}",
            serde_json::to_string(&SignatureShareWithIdentifier {
                identifier: *secret_package.identifier(),
                signature_share: sig_share,
            })
            .wrap_err("failed to serialize signature share")?
        );

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SignatureShareWithIdentifier {
    identifier: Identifier,
    signature_share: frost_ed25519::round2::SignatureShare,
}

// it's okay for all the args to end in `_path`
#[allow(clippy::struct_field_names)]
#[derive(Debug, clap::Args)]
struct Aggregate {
    /// path to the signing package
    #[arg(long)]
    signing_package_path: String,

    /// path to a file with the public key package from keygen ceremony
    #[arg(long)]
    public_key_package_path: String,

    /// optionally, path to the message bytes that were signed.
    ///
    /// if this is specified, will output the signed message as
    /// a sequencer transaction.
    #[arg(long)]
    message_path: Option<String>,

    /// optionally, path to output the signed message as a sequencer transaction.
    #[arg(long)]
    output_path: Option<String>,
}

impl Aggregate {
    async fn run(self) -> eyre::Result<()> {
        use astria_core::generated::protocol::transactions::v1alpha1::UnsignedTransaction;
        use prost::{
            Message as _,
            Name as _,
        };

        let Self {
            signing_package_path,
            public_key_package_path,
            message_path,
            output_path,
        } = self;

        let mut sig_shares: BTreeMap<Identifier, frost_ed25519::round2::SignatureShare> =
            BTreeMap::new();
        loop {
            println!("Enter signature share for a participant (or 'done' to finish)");
            let input = read_line_raw().await.wrap_err("failed to read line")?;
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
        )
        .wrap_err("failed to deserialize signing package")?;

        let public_key_package_file = std::fs::read_to_string(&public_key_package_path)
            .wrap_err(format!(
                "failed to read public key package from file: {public_key_package_path}",
            ))
            .wrap_err("failed to read public key package from file")?;
        let public_key_package =
            serde_json::from_str::<frost_ed25519::keys::PublicKeyPackage>(&public_key_package_file)
                .wrap_err("failed to deserialize public key package")?;

        let signature =
            frost_ed25519::aggregate(&signing_package, &sig_shares, &public_key_package)
                .wrap_err("failed to aggregate")?;
        println!("Aggregated signature:",);
        print!("{}", color::Fg(color::Green));
        println!("{}", hex::encode(signature.serialize()));

        if let Some(message_path) = message_path {
            let message = std::fs::read(&message_path).wrap_err("failed to read message file")?;
            let transaction = SignedTransaction {
                transaction: Some(pbjson_types::Any {
                    type_url: UnsignedTransaction::type_url(),
                    value: message.into(),
                }),
                signature: signature.serialize().to_vec().into(),
                public_key: public_key_package
                    .verifying_key()
                    .serialize()
                    .to_vec()
                    .into(),
            };

            let serialized_tx = serde_json::to_string_pretty(&transaction)
                .wrap_err("failed to serialize transaction")?;
            if let Some(output_path) = output_path {
                println!("Writing transaction to {output_path}");
                std::fs::write(output_path, serialized_tx.encode_to_vec())
                    .wrap_err("failed to write transaction to file")?;
            } else {
                println!("Signed transaction:");
                print!("{}", color::Fg(color::Green));
                println!("{serialized_tx}");
                println!("{}", color::Fg(color::Reset));
            }
        }
        Ok(())
    }
}
