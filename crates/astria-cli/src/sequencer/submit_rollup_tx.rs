use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1::{
        action::RollupDataSubmission,
        Action,
    },
};
use clap::Args;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use hex::FromHex;
use prost::bytes::Bytes;

use crate::utils::submit_transaction;

#[derive(Args, Debug)]
pub(super) struct Command {
    /// The ID of the rollup to which this transaction belongs
    #[arg(long, value_name = "BYTES")]
    rollup_id: String,

    /// The transaction data in hex format (with or without '0x' prefix)
    #[arg(long, value_name = "HEX")]
    data: String,

    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,

    /// The private key of the sequencer account sending the transaction
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    private_key: String,

    /// The URL of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,

    /// The chain ID of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,

    /// The asset to use for paying fees
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        // Store the rollup ID string for display purposes
        let rollup_id_str = self.rollup_id.clone();

        // Format the rollup ID
        let rollup_id = RollupId::from_unhashed_bytes(self.rollup_id);

        // Format the transaction data hex
        let tx_data_hex = self.data.strip_prefix("0x").unwrap_or(&self.data);
        let tx_data_bytes = Vec::from_hex(tx_data_hex)
            .wrap_err("Failed to decode transaction data from hex")?;

        println!("Submitting rollup transaction for rollup ID: {}", rollup_id_str);

        // Create the RollupDataSubmission action
        let rollup_data = RollupDataSubmission {
            rollup_id,
            data: Bytes::from(tx_data_bytes),
            fee_asset: self.fee_asset,
        };

        // Create the action
        let action = Action::RollupDataSubmission(rollup_data);

        // Submit the transaction
        let response = submit_transaction(
            &self.sequencer_url,
            self.sequencer_chain_id,
            &self.prefix,
            &self.private_key,
            action,
        )
        .await
        .wrap_err("Failed to submit rollup transaction")?;

        println!("Transaction successfully submitted!");
        println!("Transaction hash: {}", response.hash);
        println!("Included in block: {}", response.height);

        Ok(())
    }
}