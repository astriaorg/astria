use color_eyre::eyre;

mod fee_asset;
mod fee_change;
mod ibc_relayer;
mod sudo_address_change;
mod validator_update;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::IbcRelayer(ibc_relayer) => ibc_relayer.run().await,
            SubCommand::FeeAsset(fee_asset) => fee_asset.run().await,
            SubCommand::SudoAddressChange(sudo_address_change) => sudo_address_change.run().await,
            SubCommand::ValidatorUpdate(validator_update) => validator_update.run().await,
            SubCommand::FeeChange(fee_change) => fee_change.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    IbcRelayer(ibc_relayer::Command),
    FeeAsset(fee_asset::Command),
    SudoAddressChange(sudo_address_change::Command),
    ValidatorUpdate(validator_update::Command),
    FeeChange(fee_change::Command),
}
