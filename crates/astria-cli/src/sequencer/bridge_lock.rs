use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1::{
        action::BridgeLock,
        Action,
    },
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The address of the Sequencer account to lock amount to
    to_address: Address,
    /// The amount being locked
    #[arg(long)]
    amount: u128,
    #[arg(long)]
    destination_chain_address: String,
    /// The prefix to construct a bech32m address given the private key.
    #[arg(long, default_value = "astria")]
    prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    private_key: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,
    /// The asset to lock.
    #[arg(long, default_value = "nria")]
    asset: asset::Denom,
    /// The asset to pay the transfer fees with.
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::BridgeLock(BridgeLock {
                to: self.to_address,
                asset: self.asset.clone(),
                amount: self.amount,
                fee_asset: self.fee_asset.clone(),
                destination_chain_address: self.destination_chain_address.clone(),
            }),
        )
        .await
        .wrap_err("failed to submit BridgeLock transaction")?;

        println!("BridgeLock completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
