use crate::command::run;

mod fee_asset;
mod ibc_relayer;
mod sudo_address_change;
mod validator_update;

#[derive(Clone, Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) fn run(self) -> crate::command::RunCommandFut {
        match self.command {
            SubCommand::IbcRelayer(ibc_relayer) => run(|| ibc_relayer.run()),
            SubCommand::FeeAsset(fee_asset) => run(|| fee_asset.run()),
            SubCommand::SudoAddressChange(sudo_address_change) => run(|| sudo_address_change.run()),
            SubCommand::ValidatorUpdate(validator_update) => run(|| validator_update.run()),
        }
    }
}

#[derive(Clone, Debug, clap::Subcommand)]
enum SubCommand {
    IbcRelayer(ibc_relayer::Command),
    FeeAsset(fee_asset::Command),
    SudoAddressChange(sudo_address_change::Command),
    ValidatorUpdate(validator_update::Command),
}
