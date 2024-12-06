use astria_core::protocol::transaction::v1::{
    action::ValidatorUpdate,
    Action,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// The private key of the sudo account authorizing change
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
    /// The address of the Validator being updated
    #[arg(long)]
    validator_public_key: String,
    /// The power the validator is being updated to
    #[arg(long)]
    power: u32,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let verification_key = astria_core::crypto::VerificationKey::try_from(
            &*hex::decode(&self.validator_public_key)
                .wrap_err("failed to decode public key bytes from argument")?,
        )
        .wrap_err("failed to construct public key from bytes")?;
        let validator_update = ValidatorUpdate {
            power: self.power,
            verification_key,
        };

        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::ValidatorUpdate(validator_update),
        )
        .await
        .wrap_err("failed to submit ValidatorUpdate transaction")?;

        println!("ValidatorUpdate completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
