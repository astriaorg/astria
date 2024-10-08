use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::{
        action::IbcRelayerChangeAction,
        Action,
    },
};
use color_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tracing::info;

use crate::utils::submit_transaction;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Add(add) => add.run().await,
            SubCommand::Remove(remove) => remove.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    Add(Add),
    Remove(Remove),
}

#[derive(Debug, clap::Args)]
struct Add {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Add {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::IbcRelayerChange(IbcRelayerChangeAction::Addition(args.address)),
        )
        .await
        .wrap_err("failed to submit IbcRelayerChangeAction::Addition transaction")?;

        info!(height = %res.height, address = %args.address, hash = %res.hash, "IbcRelayerChangeAction::Addition completed");

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Remove {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Remove {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::IbcRelayerChange(IbcRelayerChangeAction::Removal(args.address)),
        )
        .await
        .wrap_err("failed to submit IbcRelayerChangeAction::Removal transaction")?;

        info!(height = %res.height, address = %args.address, hash = %res.hash, "IbcRelayerChangeAction::Removal completed");

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct ArgsInner {
    /// The prefix to construct a bech32m address given the private key.
    #[arg(long, default_value = "astria")]
    prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    sequencer_chain_id: String,
    /// The address to add or remove as an IBC relayer
    #[arg(long)]
    address: Address,
}
