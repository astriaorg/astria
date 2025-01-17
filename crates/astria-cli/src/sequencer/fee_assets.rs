use astria_sequencer_client::{
    HttpClient,
    SequencerClientExt as _,
};
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let SubCommand::Get(get) = self.command;
        get.run().await
    }
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Get the balance of a Sequencer account
    Get(Get),
}

#[derive(clap::Args, Debug)]
struct Get {
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
}

impl Get {
    async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_allowed_fee_assets()
            .await
            .wrap_err("failed to get fee assets")?;

        println!("Allowed fee assets:");
        for asset in res.fee_assets {
            println!("    {asset}");
        }

        Ok(())
    }
}
