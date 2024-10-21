use std::{
    fs::File,
    io::Write,
    path::PathBuf,
};

use astria_core::{
    generated::protocol::genesis::v1::{
        AddressPrefixes,
        GenesisFees,
        IbcParameters,
    },
    primitive::v1::Address,
    protocol::{
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
        genesis::v1::GenesisAppState,
    },
    Protobuf,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};

const ASTRIA_ADDRESS_PREFIX: &str = "astria";

fn alice() -> Address {
    Address::builder()
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .slice(hex::decode("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap())
        .try_build()
        .unwrap()
}

fn bob() -> Address {
    Address::builder()
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .slice(hex::decode("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a").unwrap())
        .try_build()
        .unwrap()
}

fn address_prefixes() -> AddressPrefixes {
    AddressPrefixes {
        base: "astria".into(),
        ibc_compat: "astriacompat".into(),
    }
}

#[expect(clippy::too_many_lines, reason = "all lines reasonably necessary")]
fn proto_genesis_state() -> astria_core::generated::protocol::genesis::v1::GenesisAppState {
    astria_core::generated::protocol::genesis::v1::GenesisAppState {
        address_prefixes: Some(address_prefixes()),
        authority_sudo_address: Some(alice().to_raw()),
        chain_id: "test-1".into(),
        ibc_sudo_address: Some(alice().to_raw()),
        ibc_relayer_addresses: vec![alice().to_raw(), bob().to_raw()],
        ibc_parameters: Some(IbcParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        }),
        fees: Some(GenesisFees {
            transfer: Some(
                TransferFeeComponents {
                    base: 12,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            rollup_data_submission: Some(
                RollupDataSubmissionFeeComponents {
                    base: 32,
                    multiplier: 1,
                }
                .to_raw(),
            ),
            init_bridge_account: Some(
                InitBridgeAccountFeeComponents {
                    base: 48,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            bridge_lock: Some(
                BridgeLockFeeComponents {
                    base: 12,
                    multiplier: 1,
                }
                .to_raw(),
            ),
            bridge_unlock: Some(
                BridgeUnlockFeeComponents {
                    base: 12,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            bridge_sudo_change: Some(
                BridgeSudoChangeFeeComponents {
                    base: 24,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            ics20_withdrawal: Some(
                Ics20WithdrawalFeeComponents {
                    base: 24,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            ibc_relay: Some(
                IbcRelayFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            validator_update: Some(
                ValidatorUpdateFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            fee_asset_change: Some(
                FeeAssetChangeFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            fee_change: Some(
                FeeChangeFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            ibc_relayer_change: Some(
                IbcRelayerChangeFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            sudo_address_change: Some(
                SudoAddressChangeFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
            ibc_sudo_change: Some(
                IbcSudoChangeFeeComponents {
                    base: 0,
                    multiplier: 0,
                }
                .to_raw(),
            ),
        }),
    }
}

fn genesis_state() -> GenesisAppState {
    GenesisAppState::try_from_raw(proto_genesis_state()).unwrap()
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Where to write the example genesis json (writes to stdout if unspecified).
    #[arg(long, short, value_name = "PATH")]
    output: Option<PathBuf>,
    #[arg(long, short)]
    force: bool,
}

impl Args {
    fn get_output(&self) -> Result<Box<dyn Write>> {
        match &self.output {
            Some(p) => {
                let mut opt = File::options();
                if self.force {
                    opt.write(true).truncate(true);
                } else {
                    opt.write(true).create_new(true);
                };
                opt.open(p)
                    .map(|f| Box::new(f) as Box<dyn Write>)
                    .wrap_err("failed opening provided file for writing")
            }
            None => Ok(Box::new(std::io::stdout()) as Box<dyn Write>),
        }
    }
}

/// Writes an example genesis state to a file or stdout.
///
/// # Errors
/// Returns errors if:
/// 1. the output could not be opened.
/// 2. the output could not be written to.
pub fn run(args: &Args) -> Result<()> {
    let genesis_state = genesis_state();
    let writer = args
        .get_output()
        .wrap_err("failed opening output for writing")?;
    serde_json::to_writer_pretty(writer, &genesis_state)
        .context("failed to write genesis state")?;
    Ok(())
}
