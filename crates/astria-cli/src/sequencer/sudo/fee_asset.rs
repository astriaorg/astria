use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1::{
        action::FeeAssetChange,
        Action,
    },
};
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

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

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Add Fee Asset
    Add(Add),
    /// Remove Fee Asset
    Remove(Remove),
}

#[derive(Clone, Debug, clap::Args)]
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
            Action::FeeAssetChange(FeeAssetChange::Addition(args.asset.clone())),
        )
        .await
        .wrap_err("failed to submit FeeAssetChangeAction::Addition transaction")?;

        println!("FeeAssetChangeAction::Addition completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
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
            Action::FeeAssetChange(FeeAssetChange::Removal(args.asset.clone())),
        )
        .await
        .wrap_err("failed to submit FeeAssetChangeAction::Removal transaction")?;

        println!("FeeAssetChangeAction::Removal completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct ArgsInner {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// The private key of the sudo account authorizing change
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,
    /// Asset's denomination string
    #[arg(long)]
    asset: asset::Denom,
}
