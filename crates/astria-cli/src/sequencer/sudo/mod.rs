use color_eyre::eyre;

mod fee_asset;
mod ibc_relayer;
mod sudo_address_change;
mod validator_update;

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::IbcRelayer(ibc_relayer) => ibc_relayer.run().await,
            Command::FeeAsset(fee_asset) => fee_asset.run().await,
            Command::SudoAddressChange(sudo_address_change) => sudo_address_change.run().await,
            Command::ValidatorUpdate(validator_update) => validator_update.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    IbcRelayer(ibc_relayer::Args),
    FeeAsset(fee_asset::Args),
    SudoAddressChange(sudo_address_change::Args),
    ValidatorUpdate(validator_update::Args),
}
