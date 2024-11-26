use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::{
        action::IbcSudoChange,
        Action,
    },
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
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
    /// The new address to take over sudo privileges
    #[arg(long)]
    address: Address,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::IbcSudoChange(IbcSudoChange {
                new_address: self.address,
            }),
        )
        .await
        .wrap_err("failed to submit IbcSudoChange transaction")?;

        println!("IbcSudoChange completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
