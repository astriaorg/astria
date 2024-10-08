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
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    PrepareMessage(PrepareMessage),
    Part1(Part1),
}

#[derive(Debug, clap::Args)]
struct PrepareMessage {
    /// message to be signed
    #[arg(long)]
    message: String,

    /// commitments from part1
    #[arg(long)]
    part1_commitments: String,
}

impl PrepareMessage {
    async fn run(self) -> eyre::Result<()> {
        let Self {
            message,
            part1_commitments,
        } = self;

        let commitments: BTreeMap<Identifier, frost_ed25519::round1::SigningCommitments> =
            serde_json::from_str(&part1_commitments).wrap_err("failed to parse commitments")?;

        let signing_package = frost_ed25519::SigningPackage::new(commitments, message.as_bytes());

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Part1 {
    /// index of the participant of the DKG protocol.
    /// must be 1 <= index <= n, where n is the maximum number of signers.
    #[arg(long)]
    index: u16,

    /// hex-encoded secret key package
    #[arg(long)]
    secret_key_package: String,
}

impl Part1 {
    async fn run(self) -> eyre::Result<()> {
        let mut rng = thread_rng();

        let Self {
            index,
            secret_key_package,
        } = self;

        let id: Identifier = index
            .try_into()
            .wrap_err("failed to convert index to frost identifier")?;
        println!("Our identifier is: {}", hex::encode(id.serialize()));

        let secret_package = frost_ed25519::keys::KeyPackage::deserialize(
            &hex::decode(secret_key_package).wrap_err("failed to decode secret key package")?,
        )?;
        let (nonces, commitments) =
            frost_ed25519::round1::commit(secret_package.signing_share(), &mut rng);

        println!("Our commitments are: {}", hex::encode(commitments.serialize()?));
        let mut commitments = BTreeMap::new();

        Ok(())
    }
}
