use std::{
    fs::File,
    io::Write,
    path::PathBuf,
};

use astria_core::{
    generated::{
        astria::protocol::genesis::v1::{
            AddressPrefixes,
            GenesisFees,
            IbcParameters,
        },
        price_feed::{
            marketmap::v2::{
                Market,
                MarketMap,
            },
            oracle::{
                self,
                v2::{
                    CurrencyPairGenesis,
                    QuotePrice,
                },
            },
            types::v2::CurrencyPair,
        },
    },
    primitive::v1::Address,
    protocol::{
        fees::v1::FeeComponents,
        genesis::v1::{
            Account,
            GenesisAppState,
        },
        transaction::v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeTransfer,
            BridgeUnlock,
            CurrencyPairsChange,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            MarketsChange,
            RecoverIbcClient,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use penumbra_ibc::IbcRelay;

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
    use astria_core::generated::price_feed::marketmap::v2::{
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
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![ProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "bitcoin/usd".to_string(),
                normalize_by_pair: None,
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
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![ProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "ethereum/usd".to_string(),
                normalize_by_pair: None,
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

fn proto_genesis_state() -> astria_core::generated::astria::protocol::genesis::v1::GenesisAppState {
    astria_core::generated::astria::protocol::genesis::v1::GenesisAppState {
        accounts: accounts().into_iter().map(Protobuf::into_raw).collect(),
        address_prefixes: Some(address_prefixes()),
        authority_sudo_address: Some(alice().to_raw()),
        chain_id: "test".into(),
        ibc_sudo_address: Some(alice().to_raw()),
        ibc_relayer_addresses: vec![alice().to_raw(), bob().to_raw()],
        native_asset_base_denomination: "nria".parse().unwrap(),
        ibc_parameters: Some(IbcParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        }),
        allowed_fee_assets: vec!["nria".parse().unwrap()],
        price_feed: Some(
            astria_core::generated::protocol::genesis::v1::PriceFeedGenesis {
                market_map: Some(
                    astria_core::generated::price_feed::marketmap::v2::GenesisState {
                        market_map: Some(genesis_state_markets()),
                        last_updated: 0,
                    },
                ),
                oracle: Some(oracle::v2::GenesisState {
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
        fees: Some(GenesisFees {
            transfer: Some(FeeComponents::<Transfer>::new(12, 0).to_raw()),
            rollup_data_submission: Some(
                FeeComponents::<RollupDataSubmission>::new(32, 1).to_raw(),
            ),
            init_bridge_account: Some(FeeComponents::<InitBridgeAccount>::new(48, 0).to_raw()),
            bridge_lock: Some(FeeComponents::<BridgeLock>::new(12, 1).to_raw()),
            bridge_unlock: Some(FeeComponents::<BridgeUnlock>::new(12, 0).to_raw()),
            bridge_transfer: Some(FeeComponents::<BridgeTransfer>::new(24, 0).to_raw()),
            bridge_sudo_change: Some(FeeComponents::<BridgeSudoChange>::new(24, 0).to_raw()),
            ics20_withdrawal: Some(FeeComponents::<Ics20Withdrawal>::new(24, 0).to_raw()),
            ibc_relay: Some(FeeComponents::<IbcRelay>::new(0, 0).to_raw()),
            validator_update: Some(FeeComponents::<ValidatorUpdate>::new(0, 0).to_raw()),
            fee_asset_change: Some(FeeComponents::<FeeAssetChange>::new(0, 0).to_raw()),
            fee_change: Some(FeeComponents::<FeeChange>::new(0, 0).to_raw()),
            ibc_relayer_change: Some(FeeComponents::<IbcRelayerChange>::new(0, 0).to_raw()),
            sudo_address_change: Some(FeeComponents::<SudoAddressChange>::new(0, 0).to_raw()),
            ibc_sudo_change: Some(FeeComponents::<IbcSudoChange>::new(0, 0).to_raw()),
            recover_ibc_client: Some(FeeComponents::<RecoverIbcClient>::new(0, 0).to_raw()),
            currency_pairs_change: Some(FeeComponents::<CurrencyPairsChange>::new(0, 0).to_raw()),
            markets_change: Some(FeeComponents::<MarketsChange>::new(0, 0).to_raw()),
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
