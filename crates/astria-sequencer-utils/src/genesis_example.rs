use std::{
    fs::File,
    io::Write,
    path::PathBuf,
};

use astria_core::{
    generated::{
        astria_vendored::slinky::{
            marketmap,
            marketmap::v1::{
                Market,
                MarketMap,
            },
            oracle,
            oracle::v1::{
                CurrencyPairGenesis,
                QuotePrice,
            },
            types::v1::CurrencyPair,
        },
        protocol::genesis::v1alpha1::{
            AddressPrefixes,
            IbcParameters,
        },
    },
    primitive::v1::Address,
    protocol::genesis::v1alpha1::{
        Account,
        Fees,
        GenesisAppState,
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

fn charlie() -> Address {
    Address::builder()
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .slice(hex::decode("60709e2d391864b732b4f0f51e387abb76743871").unwrap())
        .try_build()
        .unwrap()
}

fn genesis_state_markets() -> MarketMap {
    use astria_core::generated::astria_vendored::slinky::marketmap::v1::{
        ProviderConfig,
        Ticker,
    };
    use maplit::{
        btreemap,
        convert_args,
    };
    let markets = convert_args!(btreemap!(
        "BTC/USD" => Market {
            ticker: Some(Ticker {
                currency_pair: Some(CurrencyPair {
                    base: "BTC".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 3,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![ProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "bitcoin/usd".to_string(),
                normalize_by_pair: Some(CurrencyPair {
                    base: "USDT".to_string(),
                    quote: "USD".to_string(),
                }),
                invert: false,
                metadata_json: String::new(),
            }],
        },
        "ETH/USD" => Market {
            ticker: Some(Ticker {
                currency_pair: Some(CurrencyPair {
                    base: "ETH".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 3,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![ProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "ethereum/usd".to_string(),
                normalize_by_pair: Some(CurrencyPair {
                    base: "USDT".to_string(),
                    quote: "USD".to_string(),
                }),
                invert: false,
                metadata_json: String::new(),
            }],
        },
    ));
    MarketMap {
        markets,
    }
}

fn accounts() -> Vec<Account> {
    vec![
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
    ]
}

fn address_prefixes() -> AddressPrefixes {
    AddressPrefixes {
        base: "astria".into(),
        ibc_compat: "astriacompat".into(),
    }
}

fn proto_genesis_state() -> astria_core::generated::protocol::genesis::v1alpha1::GenesisAppState {
    astria_core::generated::protocol::genesis::v1alpha1::GenesisAppState {
        accounts: accounts().into_iter().map(Protobuf::into_raw).collect(),
        address_prefixes: Some(address_prefixes()),
        authority_sudo_address: Some(alice().to_raw()),
        chain_id: "test-1".into(),
        ibc_sudo_address: Some(alice().to_raw()),
        ibc_relayer_addresses: vec![alice().to_raw(), bob().to_raw()],
        native_asset_base_denomination: "nria".parse().unwrap(),
        ibc_parameters: Some(IbcParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        }),
        allowed_fee_assets: vec!["nria".parse().unwrap()],
        fees: Some(
            Fees {
                transfer_base_fee: 12,
                sequence_base_fee: 32,
                sequence_byte_cost_multiplier: 1,
                init_bridge_account_base_fee: 48,
                bridge_lock_byte_cost_multiplier: 1,
                bridge_sudo_change_fee: 24,
                ics20_withdrawal_base_fee: 24,
            }
            .into_raw(),
        ),
        slinky: Some(
            astria_core::generated::protocol::genesis::v1alpha1::SlinkyGenesis {
                market_map: Some(
                    astria_core::generated::astria_vendored::slinky::marketmap::v1::GenesisState {
                        market_map: Some(genesis_state_markets()),
                        last_updated: 0,
                        params: Some(marketmap::v1::Params {
                            market_authorities: vec![alice().to_string(), bob().to_string()],
                            admin: alice().to_string(),
                        }),
                    },
                ),
                oracle: Some(oracle::v1::GenesisState {
                    currency_pair_genesis: vec![
                        CurrencyPairGenesis {
                            id: 0,
                            nonce: 0,
                            currency_pair_price: Some(QuotePrice {
                                price: 5_834_065_777_u128.to_string(),
                                block_height: 0,
                                block_timestamp: Some(pbjson_types::Timestamp {
                                    seconds: 1_720_122_395,
                                    nanos: 0,
                                }),
                            }),
                            currency_pair: Some(CurrencyPair {
                                base: "BTC".to_string(),
                                quote: "USD".to_string(),
                            }),
                        },
                        CurrencyPairGenesis {
                            id: 1,
                            nonce: 0,
                            currency_pair_price: Some(QuotePrice {
                                price: 3_138_872_234_u128.to_string(),
                                block_height: 0,
                                block_timestamp: Some(pbjson_types::Timestamp {
                                    seconds: 1_720_122_395,
                                    nanos: 0,
                                }),
                            }),
                            currency_pair: Some(CurrencyPair {
                                base: "ETH".to_string(),
                                quote: "USD".to_string(),
                            }),
                        },
                    ],
                    next_id: 2,
                }),
            },
        ),
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
