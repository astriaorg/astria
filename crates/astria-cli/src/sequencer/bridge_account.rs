use astria_core::primitive::v1::Address;
use astria_sequencer_client::{
    HttpClient,
    SequencerClientExt,
};
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

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    /// Command for getting bridge account information
    Get(Get),
}

#[derive(Debug, clap::Args)]
struct Get {
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The address of the Sequencer bridge account
    pub(crate) address: Address,
}

impl Get {
    async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_bridge_account_info(self.address)
            .await
            .wrap_err("failed to get bridge account")?;
        let Some(info) = res.info else {
            return Err(eyre::eyre!("no bridge account information found"));
        };
        println!("Bridge Account Information for address: {}", self.address);
        println!("    Rollup Id: {}", info.rollup_id);
        println!("    Asset: {}", info.asset);
        println!("    Sudo Address: {}", info.sudo_address);
        println!("    Withdrawer Address {}", info.withdrawer_address);
        Ok(())
    }
}
