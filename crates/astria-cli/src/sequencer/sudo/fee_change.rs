use astria_core::protocol::{
    fees::v1::{
        BridgeLockFeeComponents,
        BridgeSudoChangeFeeComponents,
        BridgeUnlockFeeComponents,
        FeeAssetChangeFeeComponents,
        FeeChangeFeeComponents,
        IbcRelayFeeComponents,
        IbcRelayerChangeFeeComponents,
        IbcSudoChangeFeeComponents,
        Ics20WithdrawalFeeComponents,
        InitBridgeAccountFeeComponents,
        RollupDataSubmissionFeeComponents,
        SudoAddressChangeFeeComponents,
        TransferFeeComponents,
        ValidatorUpdateFeeComponents,
    },
    transaction::v1::{
        action::FeeChange,
        Action,
    },
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
            SubCommand::TransferFee(transfer) => transfer.run().await,
            SubCommand::InitBridgeFee(bridge_init) => bridge_init.run().await,
            SubCommand::RollupDataSubmissionFee(rollup_data) => rollup_data.run().await,
            SubCommand::BridgeLockFee(bridge_lock) => bridge_lock.run().await,
            SubCommand::BridgeUnlockFee(bridge_unlock) => bridge_unlock.run().await,
            SubCommand::BridgeSudoChangeFee(bridge_sudo_change) => bridge_sudo_change.run().await,
            SubCommand::Ics20WithdrawalFee(ics20_withdrawal) => ics20_withdrawal.run().await,
            SubCommand::IbcRelayFee(ibc_relay) => ibc_relay.run().await,
            SubCommand::IbcRelayerChangeFee(ibc_relayer_change) => ibc_relayer_change.run().await,
            SubCommand::IbcSudoChangeFee(ics_sudo_change) => ics_sudo_change.run().await,
            SubCommand::FeeAssetChangeFee(fee_asset_change) => fee_asset_change.run().await,
            SubCommand::FeeChangeFee(fee_change) => fee_change.run().await,
            SubCommand::SudoAddressChangeFee(sudo_address_change) => {
                sudo_address_change.run().await
            }
            SubCommand::ValidatorUpdateFee(validator_update) => validator_update.run().await,
        }
    }
}

#[expect(
    clippy::enum_variant_names,
    reason = "Enum variant names intentionally include 'Fee' for clarity and consistency"
)]
#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Change Transfer Fee
    TransferFee(Transfer),
    /// Change Init Bridge Account Fee
    InitBridgeFee(BridgeInit),
    /// Change Rollup Data Submission Fee
    RollupDataSubmissionFee(RollupDataSubmission),
    /// Change Bridge Lock Fee
    BridgeLockFee(BridgeLock),
    /// Change Bridge Unlock Fee
    BridgeUnlockFee(BridgeUnlock),
    /// Change Bridge Sudo Change Fee
    BridgeSudoChangeFee(BridgeSudoChange),
    /// Change ICS20 Withdrawal Fee
    Ics20WithdrawalFee(Ics20Withdrawal),
    /// Change IBC Relay Fee
    IbcRelayFee(IbcRelay),
    /// Change IBC Relayer Change Fee
    IbcRelayerChangeFee(IbcRelayerChange),
    /// Change IBC Sudo Change Fee
    IbcSudoChangeFee(IbcSudoChange),
    /// Change Fee Asset Change Fee
    FeeAssetChangeFee(FeeAssetChange),
    /// Change Fee Change Fee
    FeeChangeFee(FeeChangeFee),
    /// Change Sudo Address Change Fee
    SudoAddressChangeFee(SudoAddressChange),
    /// Change Validator Update Fee
    ValidatorUpdateFee(ValidatorUpdate),
}

#[derive(Clone, Debug, clap::Args)]
struct SudoAddressChange {
    #[command(flatten)]
    inner: ArgsInner,
}

impl SudoAddressChange {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::SudoAddressChange(
                SudoAddressChangeFeeComponents {
                    base: args.fee,
                    multiplier: args.multiplier,
                },
            )),
        )
        .await
        .wrap_err("failed to submit FeeChange::SudoAddressChange transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct IbcRelay {
    #[command(flatten)]
    inner: ArgsInner,
}

impl IbcRelay {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::IbcRelay(IbcRelayFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::IbcRelay transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct IbcRelayerChange {
    #[command(flatten)]
    inner: ArgsInner,
}

impl IbcRelayerChange {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::IbcRelayerChange(IbcRelayerChangeFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::IbcRelayerChange transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct IbcSudoChange {
    #[command(flatten)]
    inner: ArgsInner,
}

impl IbcSudoChange {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::IbcSudoChange(IbcSudoChangeFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::IbcSudoChange transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct FeeAssetChange {
    #[command(flatten)]
    inner: ArgsInner,
}

impl FeeAssetChange {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::FeeAssetChange(FeeAssetChangeFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::FeeAssetChange transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct FeeChangeFee {
    #[command(flatten)]
    inner: ArgsInner,
}

impl FeeChangeFee {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::FeeChange(FeeChangeFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::FeeChangeFee transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct ValidatorUpdate {
    #[command(flatten)]
    inner: ArgsInner,
}

impl ValidatorUpdate {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::ValidatorUpdate(ValidatorUpdateFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::ValidatorUpdate transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
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
            Action::FeeChange(FeeChange::Transfer(TransferFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::Transfer transaction")?;

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
            Action::FeeChange(FeeChange::InitBridgeAccount(
                InitBridgeAccountFeeComponents {
                    base: args.fee,
                    multiplier: args.multiplier,
                },
            )),
        )
        .await
        .wrap_err("failed to submit FeeChange::InitBridgeAccount transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct RollupDataSubmission {
    #[command(flatten)]
    inner: ArgsInner,
}

impl RollupDataSubmission {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::RollupDataSubmission(
                RollupDataSubmissionFeeComponents {
                    base: args.fee,
                    multiplier: args.multiplier,
                },
            )),
        )
        .await
        .wrap_err("failed to submit FeeChange::RollupDataSubmission")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct BridgeLock {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BridgeLock {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::BridgeLock(BridgeLockFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::BridgeLock transaction")?;

        println!("FeeChangeAction completed!");
        println!("Included in block: {}", res.height);
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
struct BridgeUnlock {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BridgeUnlock {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let res = submit_transaction(
            args.sequencer_url.as_str(),
            args.sequencer_chain_id.clone(),
            &args.prefix,
            args.private_key.as_str(),
            Action::FeeChange(FeeChange::BridgeUnlock(BridgeUnlockFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange:BridgeUnlock transaction")?;

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
            Action::FeeChange(FeeChange::BridgeSudoChange(BridgeSudoChangeFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::BridgeSudoChange transaction")?;

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
            Action::FeeChange(FeeChange::Ics20Withdrawal(Ics20WithdrawalFeeComponents {
                base: args.fee,
                multiplier: args.multiplier,
            })),
        )
        .await
        .wrap_err("failed to submit FeeChange::Ics20Withdrawal transaction")?;

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
    /// The new base fee
    #[arg(long)]
    pub(crate) fee: u128,
    /// The new multiplier fee
    #[arg(long)]
    pub(crate) multiplier: u128,
}
