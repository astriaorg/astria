use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1::{
        action::InitBridgeAccount,
        Action,
    },
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// The private key of the account initializing the bridge account
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
    /// The authorized withdrawer address for this account.
    /// If unset, the sender address will be used.
    /// Should be an astria-prefixed bech32m address.
    /// Ex: "astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr"
    #[arg(long)]
    withdrawer_address: Option<Address>,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(long = "sequencer.chain-id", env = "ROLLUP_SEQUENCER_CHAIN_ID")]
    sequencer_chain_id: String,
    /// Plaintext rollup name (to be hashed into a rollup ID)
    /// to initialize the bridge account with.
    #[arg(long)]
    rollup_name: String,
    /// The asset to transer.
    #[arg(long, default_value = "nria")]
    asset: asset::Denom,
    /// The asset to pay the transfer fees with.
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        use astria_core::primitive::v1::RollupId;

        let rollup_id = RollupId::from_unhashed_bytes(self.rollup_name.as_bytes());
        let res = crate::utils::submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::InitBridgeAccount(InitBridgeAccount {
                rollup_id,
                asset: self.asset.clone(),
                fee_asset: self.fee_asset.clone(),
                sudo_address: None,
                withdrawer_address: self.withdrawer_address,
            }),
        )
        .await
        .wrap_err("failed to submit InitBridgeAccount transaction")?;

        println!("InitBridgeAccount completed!");
        println!("Included in block: {}", res.height);
        println!("Rollup name: {}", self.rollup_name);
        println!("Rollup ID: {rollup_id}");
        Ok(())
    }
}
