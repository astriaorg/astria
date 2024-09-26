use astria_sequencer_client::{
    Client as _,
    HttpClient,
};
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let Command::Get(get) = self.command;
        get.run().await
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get the current block height of the Sequencer node
    Get(GetArgs),
}

#[derive(clap::Args, Debug)]
struct GetArgs {
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
}

impl GetArgs {
    async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .latest_block()
            .await
            .wrap_err("failed to get cometbft block")?;

        println!("Block Height:");
        println!("    {}", res.block.header.height);

        Ok(())
    }
}
