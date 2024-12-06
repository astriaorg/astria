use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1::{
        action::Transfer,
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
    /// The address of the Sequencer account to send amount to
    to_address: Address,
    /// The amount being sent
    #[arg(long)]
    amount: u128,
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// The private key of account being sent from
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,
    /// The asset to transer.
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
            Action::Transfer(Transfer {
                to: self.to_address,
                amount: self.amount,
                asset: self.asset.clone(),
                fee_asset: self.fee_asset.clone(),
            }),
        )
        .await
        .wrap_err("failed to submit transfer transaction")?;

        println!("Transfer completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
