use clap::Subcommand;
use color_eyre::eyre;

/// Interact with a Sequencer node
// allow: these are one-shot variants. the size doesn't matter as they are
// passed around only once.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Commands for interacting with Sequencer accounts
    CollectWithdrawals(crate::commands::bridge::collect::WithdrawalEvents),
    SubmitWithdrawals(crate::commands::bridge::submit::WithdrawalEvents),
}

impl Command {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        match self {
            Command::CollectWithdrawals(args) => args.run().await,
            Command::SubmitWithdrawals(args) => args.run().await,
        }
    }
}
