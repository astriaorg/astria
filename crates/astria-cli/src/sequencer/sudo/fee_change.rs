use astria_core::protocol::transaction::v1alpha1::{
    action::{
        FeeChange,
        FeeChangeAction,
    },
    Action,
};
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::submit_transaction;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::TransferBaseFee(transfer) => transfer.run().await,
            SubCommand::InitBridgeBaseFee(bridge_init) => bridge_init.run().await,
            SubCommand::SequenceBaseFee(sequence_base) => sequence_base.run().await,
            SubCommand::SequenceByteCostMul(sequence_byte_cost_mul) => {
                sequence_byte_cost_mul.run().await
            }
            SubCommand::BridgeLockByteCostMul(bridge_lock_byte_cost_mul) => {
                bridge_lock_byte_cost_mul.run().await
            }
            SubCommand::BridgeSudoChangeBaseFee(bridge_sudo_change) => {
                bridge_sudo_change.run().await
            }
            SubCommand::Ics20WithdrawalBaseFee(ics20_withdrawal) => ics20_withdrawal.run().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Chnage Transfer Base Fee
    TransferBaseFee(Transfer),
    /// Change Init Bridge Account Base Fee
    InitBridgeBaseFee(BridgeInit),
    /// Change Sequence Base Fee
    SequenceBaseFee(SequenceBaseFee),
    /// Change Sequence Byte Cost Multiplier
    SequenceByteCostMul(SequenceByteCostMul),
    /// Change Bridge Lock Byte Cost Multiplier
    BridgeLockByteCostMul(BridgeLockByteCostMul),
    /// Change Bridge Sudo Change Base Fee
    BridgeSudoChangeBaseFee(BridgeSudoChange),
    /// Change ICS20 Withdrawal Base Fee
    Ics20WithdrawalBaseFee(Ics20Withdrawal),
}

#[derive(Clone, Debug, clap::Args)]
struct Transfer {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Transfer {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::TransferBaseFee,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::TransferBaseFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct BridgeInit {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BridgeInit {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::InitBridgeAccountBaseFee,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::InitBridgeAccountBaseFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct SequenceBaseFee {
    #[command(flatten)]
    inner: ArgsInner,
}

impl SequenceBaseFee {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::SequenceBaseFee,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::SequenceBaseFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct SequenceByteCostMul {
    #[command(flatten)]
    inner: ArgsInner,
}

impl SequenceByteCostMul {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::SequenceByteCostMultiplier,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::SequenceByteCostMultiplier transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct BridgeLockByteCostMul {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BridgeLockByteCostMul {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::BridgeLockByteCostMultiplier,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::BridgeLockByteCostMultiplier transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct BridgeSudoChange {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BridgeSudoChange {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::BridgeSudoChangeBaseFee,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::BridgeSudoChangeBaseFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct Ics20Withdrawal {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Ics20Withdrawal {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::Ics20WithdrawalBaseFee,
                new_value: args.fee,
            }),
        )
        .await
        .wrap_err("failed to submit FeeChangeAction::Ics20WithdrawalBaseFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct ArgsInner {
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
    /// The new fee value
    #[arg(long)]
    pub(crate) fee: u128,
}
