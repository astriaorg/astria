use clap::{
    Args,
    Subcommand,
};

/// Interact with the Sequencer
#[derive(Subcommand)]
pub enum Command {
    /// Create a new Sequencer account
    Account {
        #[clap(subcommand)]
        command: AccountCommand,
    },
    /// Get the balance of a Sequencer account
    Balance {
        #[clap(subcommand)]
        command: BalanceCommand,
    },
}

#[derive(Subcommand)]
pub enum AccountCommand {
    /// Create a new sequencer account
    Create,
}

#[derive(Subcommand)]
pub enum BalanceCommand {
    /// Get the balance of a sequencer account
    Get(BalanceGetArgs),
}

#[derive(Args, Debug)]
pub struct BalanceGetArgs {
    /// The address of the sequencer account
    #[clap(long)]
    pub(crate) address: Option<String>,
}
