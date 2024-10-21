use astria_core::{
    primitive::v1::{
        Address,
        asset,
    },
    protocol::{
        fees::v1::TransferFeeComponents,
        transaction::v1::{
            Action,
            action::FeeChange::{
                self,
                Transfer,
            },
        },
    },
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(clap::Args, Debug)]
pub(super) struct Command {
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
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::DEFAULT_SEQUENCER_CHAIN_ID
    )]
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
        let action = Action::FeeChange(FeeChange::Transfer(TransferFeeComponents {
            base: 12,
            multiplier: 1,
        }));
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            action,
        )
        .await
        .wrap_err("failed to submit change transfer fee transaction")?;

        println!("Transfer completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
