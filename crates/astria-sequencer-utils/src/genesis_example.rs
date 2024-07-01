use std::{
    fs::File,
    io::Write,
    path::PathBuf,
};

use astria_core::{
    primitive::v1::Address,
    sequencer::{
        Account,
        AddressPrefixes,
        Fees,
        GenesisState,
        IBCParameters,
        UncheckedGenesisState,
    },
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

fn charlie() -> Address {
    Address::builder()
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .slice(hex::decode("60709e2d391864b732b4f0f51e387abb76743871").unwrap())
        .try_build()
        .unwrap()
}

fn genesis_state() -> GenesisState {
    UncheckedGenesisState {
        accounts: vec![
            Account {
                address: alice(),
                balance: 1_000_000_000_000_000_000,
            },
            Account {
                address: bob(),
                balance: 1_000_000_000_000_000_000,
            },
            Account {
                address: charlie(),
                balance: 1_000_000_000_000_000_000,
            },
        ],
        address_prefixes: AddressPrefixes {
            base: "astria".into(),
        },
        authority_sudo_address: alice(),
        ibc_sudo_address: alice(),
        ibc_relayer_addresses: vec![alice(), bob()],
        native_asset_base_denomination: "nria".to_string(),
        ibc_params: IBCParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        },
        allowed_fee_assets: vec!["nria".parse().unwrap()],
        fees: Fees {
            transfer_base_fee: 12,
            sequence_base_fee: 32,
            sequence_byte_cost_multiplier: 1,
            init_bridge_account_base_fee: 48,
            bridge_lock_byte_cost_multiplier: 1,
            bridge_sudo_change_fee: 24,
            ics20_withdrawal_base_fee: 24,
        },
    }
    .try_into()
    .unwrap()
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
