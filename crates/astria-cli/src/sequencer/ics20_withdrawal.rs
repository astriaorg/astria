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
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use ibc_types::core::{
    channel::ChannelId,
    client::Height,
};
use tracing::info;

use crate::utils::{
    address_from_signing_key,
    signing_key_from_private_key,
    submit_transaction,
};

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
    /// The transfer amount to send
    #[arg(long)]
    amount: u128,
    /// The address on the destination chain
    #[arg(long)]
    destination_chain_address: String,
    /// The source channel used for withdrawal
    #[arg(long)]
    source_channel: String,
    /// The address to refund on timeout, if unset refunds the signer
    #[arg(long)]
    return_address: Option<Address>,
    /// An optional memo to send with transaction
    #[arg(long)]
    memo: Option<String>,
    /// The bridge account to transfer from
    #[arg(long)]
    bridge_address: Option<Address>,
    /// Use compatibility address format (for example: when sending USDC to Noble)
    #[arg(long)]
    compat: bool,
    /// The prefix to construct a bech32m address given the private key
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
    /// The asset to withdraw
    #[arg(long, default_value = "nria")]
    asset: asset::Denom,
    /// The asset to be used to pay the fees
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
}

impl Command {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let signing_key = signing_key_from_private_key(&self.private_key)?;
        let from_address = address_from_signing_key(&signing_key, &self.prefix)?;
        let res = submit_transaction(
            self.sequencer_url.as_str(),
            self.sequencer_chain_id.clone(),
            &self.prefix,
            self.private_key.as_str(),
            Action::Ics20Withdrawal(Ics20Withdrawal {
                amount: self.amount,
                denom: self.asset,
                destination_chain_address: self.destination_chain_address,
                return_address: self.return_address.unwrap_or(from_address),
                timeout_height: Height {
                    revision_number: u64::MAX,
                    revision_height: u64::MAX,
                },
                timeout_time: now_plus_5_minutes(),
                source_channel: ChannelId(self.source_channel),
                fee_asset: self.fee_asset,
                memo: self.memo.unwrap_or_default(),
                bridge_address: self.bridge_address,
                use_compat_address: self.compat,
            }),
        )
        .await
        .wrap_err("failed to perform ics20 withdrawal")?;

        info!(hash = %res.hash, at_height = %res.height, "ics20 withdrawal completed");

        Ok(())
    }
}
