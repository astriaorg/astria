use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1alpha1::{
        action::Ics20Withdrawal,
        Action,
    },
};
use clap::ArgAction;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use ibc_types::core::{
    channel::ChannelId,
    client::Height,
};

use crate::utils::submit_transaction;

fn now_plus_5_minutes() -> u64 {
    use std::time::Duration;
    tendermint::Time::now()
        .checked_add(Duration::from_secs(300))
        .expect("adding 5 minutes to the current time should never fail")
        .unix_timestamp_nanos()
        .try_into()
        .expect("timestamp must be positive, so this conversion would only fail if negative")
}

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    #[arg(long)]
    pub(crate) amount: u128,
    #[arg(long)]
    pub(crate) destination_chain_address: String,
    /// The source channel used for withdrawal
    #[arg(long)]
    pub(crate) source_channel: String,
    /// The address to refund on timeout, if unset refunds the signer
    #[arg(long)]
    pub(crate) return_address: Address,
    /// A memo to send with transaction
    #[arg(long, default_value = "")]
    pub(crate) memo: String,
    /// The bridge account to transfer from
    #[arg(long, default_value = None)]
    pub(crate) bridge_address: Option<Address>,
    #[arg(long, action(ArgAction::SetTrue))]
    pub(crate) use_compact: bool,
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
    #[arg(
         long,
         env = "SEQUENCER_URL",
         default_value = crate::DEFAULT_SEQUENCER_RPC
     )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
         long = "sequencer.chain-id",
         env = "ROLLUP_SEQUENCER_CHAIN_ID",
         default_value = crate::DEFAULT_SEQUENCER_CHAIN_ID
     )]
    pub(crate) sequencer_chain_id: String,
    /// The asset to lock.
    #[arg(long, default_value = "nria")]
    pub(crate) asset: asset::Denom,
    /// The asset to pay the transfer fees with.
    #[arg(long, default_value = "nria")]
    pub(crate) fee_asset: asset::Denom,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        println!("compact is: {}", self.use_compact);
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::Ics20Withdrawal(Ics20Withdrawal {
                amount: self.amount,
                denom: self.asset,
                destination_chain_address: self.destination_chain_address,
                return_address: self.return_address,
                timeout_height: Height {
                    revision_number: u64::MAX,
                    revision_height: u64::MAX,
                },
                timeout_time: now_plus_5_minutes(),
                source_channel: ChannelId(self.source_channel),
                fee_asset: self.fee_asset,
                memo: self.memo,
                bridge_address: self.bridge_address,
                use_compat_address: self.use_compact,
            }),
        )
        .await
        .wrap_err("failed to submit BridgeLock transaction")?;

        println!("BridgeLock completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}
