//! Utilities to create objects used in various tests of the Astria codebase.
#![expect(clippy::missing_panics_doc, reason = "these are test-only functions")]

use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    sync::Arc,
};

use astria_core_address::Address;
use bytes::Bytes;
use indexmap::IndexMap;
use prost::Message as _;
use tendermint::abci::types::ExtendedCommitInfo;

use super::{
    group_rollup_data_submissions_in_signed_transaction_transactions_by_rollup_id,
    transaction::v1::action::RollupDataSubmission,
};
use crate::{
    crypto::SigningKey,
    generated::{
        price_feed::{
            marketmap::v2::{
                GenesisState as RawMarketMapGenesisState,
                Market as RawMarket,
                MarketMap as RawMarketMap,
                Params as RawMarketMapParams,
                ProviderConfig as RawProviderConfig,
                Ticker as RawTicker,
            },
            oracle::v2::{
                CurrencyPairGenesis as RawCurrencyPairGenesis,
                GenesisState as RawOracleGenesisState,
                QuotePrice as RawQuotePrice,
            },
            types::v2::CurrencyPair as RawCurrencyPair,
        },
        protocol::genesis::v1::PriceFeedGenesis as RawPriceFeedGenesis,
    },
    primitive::v1::{
        derive_merkle_tree_from_rollup_txs,
        RollupId,
    },
    protocol::{
        genesis::v1::PriceFeedGenesis,
        price_feed::v1::ExtendedCommitInfoWithCurrencyPairMapping,
        transaction::v1::TransactionBody,
    },
    sequencerblock::v1::{
        block::{
            self,
            Deposit,
            ExpandedBlockData,
            SequencerBlockBuilder,
        },
        DataItem,
        SequencerBlock,
    },
    upgrades::{
        test_utils::UpgradesBuilder,
        v1::{
            Change,
            ChangeHash,
        },
    },
    Protobuf as _,
};

#[derive(Default)]
pub struct UnixTimeStamp {
    pub secs: i64,
    pub nanos: u32,
}

impl From<(i64, u32)> for UnixTimeStamp {
    fn from(val: (i64, u32)) -> Self {
        Self {
            secs: val.0,
            nanos: val.1,
        }
    }
}

/// Allows configuring a Comet BFT block setting the height, signing key and
/// proposer address.
///
/// If the proposer address is not set it will be generated from the signing key.
pub struct ConfigureSequencerBlock {
    pub block_hash: Option<block::Hash>,
    pub chain_id: Option<String>,
    pub height: u32,
    pub proposer_address: Option<tendermint::account::Id>,
    pub signing_key: Option<SigningKey>,
    pub sequence_data: Vec<(RollupId, Vec<u8>)>,
    pub deposits: Vec<Deposit>,
    pub unix_timestamp: UnixTimeStamp,
    pub use_data_items: bool,
    pub with_aspen: bool,
    pub with_extended_commit_info: bool,
}

impl Default for ConfigureSequencerBlock {
    fn default() -> Self {
        Self {
            block_hash: None,
            chain_id: None,
            height: 0,
            proposer_address: None,
            signing_key: None,
            sequence_data: vec![],
            deposits: vec![],
            unix_timestamp: UnixTimeStamp::default(),
            use_data_items: true,
            with_aspen: true,
            with_extended_commit_info: true,
        }
    }
}

impl ConfigureSequencerBlock {
    /// Construct a [`SequencerBlock`] with the configured parameters.
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        clippy::too_many_lines,
        reason = "This should only be used in tests, so everything here is unwrapped"
    )]
    pub fn make(self) -> SequencerBlock {
        use tendermint::Time;

        use crate::{
            protocol::transaction::v1::Action,
            sequencerblock::v1::block::RollupData,
        };

        let Self {
            block_hash,
            chain_id,
            height,
            signing_key,
            proposer_address,
            sequence_data,
            unix_timestamp,
            deposits,
            use_data_items,
            with_aspen,
            with_extended_commit_info,
        } = self;

        let block_hash = block_hash.unwrap_or_else(|| block::Hash::new([0; 32]));
        let chain_id = chain_id.unwrap_or_else(|| "test".to_string());

        let signing_key = signing_key.unwrap_or_else(|| SigningKey::new(rand::rngs::OsRng));

        let proposer_address = proposer_address.unwrap_or_else(|| {
            let public_key: tendermint::crypto::ed25519::VerificationKey =
                signing_key.verification_key().as_ref().try_into().unwrap();
            tendermint::account::Id::from(public_key)
        });

        let actions: Vec<Action> = sequence_data
            .into_iter()
            .map(|(rollup_id, data)| {
                RollupDataSubmission {
                    rollup_id,
                    data: data.into(),
                    fee_asset: "nria".parse().unwrap(),
                }
                .into()
            })
            .collect();
        let txs = if actions.is_empty() {
            vec![]
        } else {
            let body = TransactionBody::builder()
                .actions(actions)
                .chain_id(chain_id.clone())
                .nonce(1)
                .try_build()
                .expect(
                    "should be able to build transaction body since only rollup data submission \
                     actions are contained",
                );
            vec![Arc::new(body.sign(&signing_key))]
        };
        let mut deposits_map: HashMap<RollupId, Vec<Deposit>> = HashMap::new();
        for deposit in deposits {
            if let Some(entry) = deposits_map.get_mut(&deposit.rollup_id) {
                entry.push(deposit);
            } else {
                deposits_map.insert(deposit.rollup_id, vec![deposit]);
            }
        }

        let mut rollup_transactions =
            group_rollup_data_submissions_in_signed_transaction_transactions_by_rollup_id(&txs);
        for (rollup_id, deposit) in deposits_map.clone() {
            rollup_transactions
                .entry(rollup_id)
                .or_default()
                .extend(deposit.into_iter().map(|deposit| {
                    RollupData::Deposit(Box::new(deposit))
                        .into_raw()
                        .encode_to_vec()
                        .into()
                }));
        }
        rollup_transactions.sort_unstable_keys();
        let rollup_transactions_tree = derive_merkle_tree_from_rollup_txs(&rollup_transactions);

        let rollup_ids_root = merkle::Tree::from_leaves(
            rollup_transactions
                .keys()
                .map(|rollup_id| rollup_id.as_ref().to_vec()),
        )
        .root();

        let mut data = if use_data_items {
            vec![
                DataItem::RollupTransactionsRoot(rollup_transactions_tree.root()).encode(),
                DataItem::RollupIdsRoot(rollup_ids_root).encode(),
            ]
        } else {
            vec![
                rollup_transactions_tree.root().to_vec().into(),
                rollup_ids_root.to_vec().into(),
            ]
        };

        if with_aspen {
            assert!(
                use_data_items,
                "can't include aspen upgrade and also use legacy form of data/txns"
            );
            data.push(upgrade_change_hashes_bytes());
        }

        if with_extended_commit_info {
            assert!(
                use_data_items,
                "can't include aspen upgrade and also include extended commit info"
            );
            data.push(minimal_extended_commit_info_bytes());
        }

        data.extend(txs.into_iter().map(|tx| tx.to_raw().encode_to_vec().into()));
        let expanded_block_data =
            ExpandedBlockData::new_from_typed_data(&data, with_extended_commit_info).unwrap();

        SequencerBlockBuilder {
            block_hash,
            chain_id: chain_id.try_into().unwrap(),
            height: height.into(),
            time: Time::from_unix_timestamp(unix_timestamp.secs, unix_timestamp.nanos).unwrap(),
            proposer_address,
            expanded_block_data,
            deposits: deposits_map,
        }
        .try_build()
        .unwrap()
    }
}

/// Returns the change hashes of `Aspen`.
pub fn upgrade_change_hashes() -> Vec<ChangeHash> {
    let upgrades = UpgradesBuilder::new().build();
    upgrades
        .aspen()
        .unwrap()
        .changes()
        .map(Change::calculate_hash)
        .collect()
}

#[must_use]
pub fn upgrade_change_hashes_bytes() -> Bytes {
    DataItem::UpgradeChangeHashes(upgrade_change_hashes()).encode()
}

#[must_use]
pub fn minimal_extended_commit_info() -> ExtendedCommitInfoWithCurrencyPairMapping {
    let extended_commit_info = ExtendedCommitInfo {
        round: 0u16.into(),
        votes: vec![],
    };
    ExtendedCommitInfoWithCurrencyPairMapping {
        extended_commit_info,
        id_to_currency_pair: IndexMap::new(),
    }
}

#[must_use]
pub fn minimal_extended_commit_info_bytes() -> Bytes {
    DataItem::ExtendedCommitInfo(
        minimal_extended_commit_info()
            .into_raw()
            .encode_to_vec()
            .into(),
    )
    .encode()
}

#[must_use]
pub fn dummy_price_feed_genesis() -> PriceFeedGenesis {
    let mut markets = BTreeMap::new();
    markets.insert(
        "BTC/USD".to_string(),
        RawMarket {
            ticker: Some(RawTicker {
                currency_pair: Some(RawCurrencyPair {
                    base: "BTC".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![RawProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "bitcoin/usd".to_string(),
                normalize_by_pair: None,
                invert: false,
                metadata_json: String::new(),
            }],
        },
    );
    markets.insert(
        "ETH/USD".to_string(),
        RawMarket {
            ticker: Some(RawTicker {
                currency_pair: Some(RawCurrencyPair {
                    base: "ETH".to_string(),
                    quote: "USD".to_string(),
                }),
                decimals: 8,
                min_provider_count: 1,
                enabled: true,
                metadata_json: String::new(),
            }),
            provider_configs: vec![RawProviderConfig {
                name: "coingecko_api".to_string(),
                off_chain_ticker: "ethereum/usd".to_string(),
                normalize_by_pair: None,
                invert: false,
                metadata_json: String::new(),
            }],
        },
    );

    let price_feed_genesis = RawPriceFeedGenesis {
        market_map: Some(RawMarketMapGenesisState {
            market_map: Some(RawMarketMap {
                markets,
            }),
            last_updated: 0,
            params: Some(RawMarketMapParams {
                market_authorities: vec![alice().to_string(), bob().to_string()],
                admin: alice().to_string(),
            }),
        }),
        oracle: Some(RawOracleGenesisState {
            currency_pair_genesis: vec![
                RawCurrencyPairGenesis {
                    id: 0,
                    nonce: 0,
                    currency_pair_price: Some(RawQuotePrice {
                        price: 5_834_065_777_u128.to_string(),
                        block_height: 0,
                        block_timestamp: Some(pbjson_types::Timestamp {
                            seconds: 1_720_122_395,
                            nanos: 0,
                        }),
                    }),
                    currency_pair: Some(RawCurrencyPair {
                        base: "BTC".to_string(),
                        quote: "USD".to_string(),
                    }),
                },
                RawCurrencyPairGenesis {
                    id: 1,
                    nonce: 0,
                    currency_pair_price: Some(RawQuotePrice {
                        price: 3_138_872_234_u128.to_string(),
                        block_height: 0,
                        block_timestamp: Some(pbjson_types::Timestamp {
                            seconds: 1_720_122_395,
                            nanos: 0,
                        }),
                    }),
                    currency_pair: Some(RawCurrencyPair {
                        base: "ETH".to_string(),
                        quote: "USD".to_string(),
                    }),
                },
            ],
            next_id: 2,
        }),
    };
    PriceFeedGenesis::try_from_raw(price_feed_genesis).unwrap()
}

fn alice() -> Address {
    Address::builder()
        .prefix("astria")
        .slice(hex::decode("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap())
        .try_build()
        .unwrap()
}

fn bob() -> Address {
    Address::builder()
        .prefix("astria")
        .slice(hex::decode("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a").unwrap())
        .try_build()
        .unwrap()
}
