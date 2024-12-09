use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1::{
        action::BridgeSudoChange,
        Action,
    },
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(clap::Args, Debug)]
#[command(group(clap::ArgGroup::new("new_address")
    .required(true)
    .multiple(true)
    .args(&["new_sudo_address", "new_withdrawer_address"])))]
pub(crate) struct Command {
    /// The bridge account whose privileges will be modified.
    pub(crate) bridge_address: Address,
    /// The new address to receive sudo privileges.
    #[arg(long, default_value = None)]
    pub(crate) new_sudo_address: Option<Address>,
    /// The new address to receive withdrawer privileges.
    #[arg(long, default_value = None)]
    pub(crate) new_withdrawer_address: Option<Address>,
    /// The prefix to construct a bech32m address given the private key.
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    pub(crate) sequencer_chain_id: String,
    /// The asset to pay the transfer fees with.
    #[arg(long, default_value = "nria")]
    pub(crate) fee_asset: asset::Denom,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::BridgeSudoChange(BridgeSudoChange {
                bridge_address: self.bridge_address,
                new_sudo_address: self.new_sudo_address,
                new_withdrawer_address: self.new_withdrawer_address,
                fee_asset: self.fee_asset.clone(),
            }),
        )
        .await
        .wrap_err("failed to submit BridgeSudoChange transaction")?;

        println!("BridgeSudoChange completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
