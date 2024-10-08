use color_eyre::eyre;

mod dkg;
mod sign;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Dkg(dkg) => dkg.run().await,
            SubCommand::Sign(sign) => sign.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    Dkg(dkg::Command),
    Sign(sign::Command),
}
