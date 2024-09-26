use astria_core::primitive::v1::Address;
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
    /// Get the balance of a Sequencer account
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
    /// The address of the Sequencer account
    address: Address,
}

impl GetArgs {
    async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_latest_balance(self.address)
            .await
            .wrap_err("failed to get balance")?;

        println!("Balances for address: {}", self.address);
        for balance in res.balances {
            println!("    {} {}", balance.balance, balance.denom);
        }

        Ok(())
    }
}
