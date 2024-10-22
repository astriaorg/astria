use color_eyre::eyre;

#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Transfer(transfer) => transfer.run().await,
            SubCommand::BridgeLock(bridge_lock) => bridge_lock.run().await,
            SubCommand::InitBridgeAccount(init_bridge_account) => init_bridge_account.run().await,
            SubCommand::Ics20Withdrawal(ics20_withdrawal) => ics20_withdrawal.run().await,
            SubCommand::IbcRelayer(ibc_relayer) => ibc_relayer.run().await,
            SubCommand::FeeAsset(fee_asset) => fee_asset.run().await,
            SubCommand::SudoAddressChange(sudo_address_change) => sudo_address_change.run().await,
            SubCommand::ValidatorUpdate(validator_update) => validator_update.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub(super) enum SubCommand {
    Transfer(transfer::Command),
    BridgeLock(bridge_lock::Command),
    InitBridgeAccount(init_bridge_account::Command),
    Ics20Withdrawal(ics20_withdrawal::Command),
    IbcRelayer(ibc_relayer_change::Command),
    FeeAsset(fee_asset_change::Command),
    SudoAddressChange(sudo_address_change::Command),
    ValidatorUpdate(validator_update::Command),
}

mod transfer {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::TransferFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
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

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod bridge_lock {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::BridgeLockFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action = Action::FeeChange(FeeChange::BridgeLock(BridgeLockFeeComponents {
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
            .wrap_err("failed to submit change bridge lock fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod init_bridge_account {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::InitBridgeAccountFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action = Action::FeeChange(FeeChange::InitBridgeAccount(
                InitBridgeAccountFeeComponents {
                    base: 12,
                    multiplier: 1,
                },
            ));
            let res = submit_transaction(
                self.sequencer_url.as_str(),
                self.sequencer_chain_id.clone(),
                &self.prefix,
                self.private_key.as_str(),
                action,
            )
            .await
            .wrap_err("failed to submit change init bridge account fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod ics20_withdrawal {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::Ics20WithdrawalFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action =
                Action::FeeChange(FeeChange::Ics20Withdrawal(Ics20WithdrawalFeeComponents {
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
            .wrap_err("failed to submit change ics20 withdrawal fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod ibc_relayer_change {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::IbcRelayerChangeFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action =
                Action::FeeChange(FeeChange::IbcRelayerChange(IbcRelayerChangeFeeComponents {
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
            .wrap_err("failed to submit change ibc relayer change fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod fee_asset_change {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::FeeAssetChangeFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action =
                Action::FeeChange(FeeChange::FeeAssetChange(FeeAssetChangeFeeComponents {
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
            .wrap_err("failed to submit change fee asset change fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod sudo_address_change {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::SudoAddressChangeFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action = Action::FeeChange(FeeChange::SudoAddressChange(
                SudoAddressChangeFeeComponents {
                    base: 12,
                    multiplier: 1,
                },
            ));
            let res = submit_transaction(
                self.sequencer_url.as_str(),
                self.sequencer_chain_id.clone(),
                &self.prefix,
                self.private_key.as_str(),
                action,
            )
            .await
            .wrap_err("failed to submit change sudo address change fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}

mod validator_update {
    use astria_core::{
        primitive::v1::asset,
        protocol::{
            fees::v1::ValidatorUpdateFeeComponents,
            transaction::v1::{
                action::FeeChange,
                Action,
            },
        },
    };
    use color_eyre::eyre::{
        self,
        Context as _,
    };

    use crate::utils::submit_transaction;

    #[derive(clap::Args, Debug)]
    pub(crate) struct Command {
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
        pub(crate) async fn run(self) -> eyre::Result<()> {
            let action =
                Action::FeeChange(FeeChange::ValidatorUpdate(ValidatorUpdateFeeComponents {
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
            .wrap_err("failed to submit change validator update fee transaction")?;

            println!("Fee Change completed!");
            println!("Included in block: {}", res.height);
            Ok(())
        }
    }
}
