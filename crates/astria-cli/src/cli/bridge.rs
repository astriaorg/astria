use clap::Subcommand;
use color_eyre::eyre;

/// Interact with a Sequencer node
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Commands for interacting with Sequencer accounts
    CollectWithdrawalEvents(crate::commands::bridge::CollectWithdrawalEvents),
}

impl Command {
    pub async fn run(self) -> eyre::Result<()> {
        match self {
            Command::CollectWithdrawalEvents(args) => args.run().await,
        }
    }
}
