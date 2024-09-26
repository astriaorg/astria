pub(crate) mod collect;
pub(crate) mod submit;

use clap::Subcommand;
use color_eyre::eyre;

/// Interact with a Sequencer node
#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::CollectWithdrawals(args) => args.run().await,
            Command::SubmitWithdrawals(args) => args.run().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Commands for interacting with Sequencer accounts
    CollectWithdrawals(collect::WithdrawalEventsArgs),
    SubmitWithdrawals(submit::WithdrawalEventsArgs),
}
