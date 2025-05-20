use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        Arc,
        LazyLock,
    },
    time::Duration,
};

use astria_core::{
    crypto::{
        SigningKey,
        ADDRESS_LENGTH,
    },
    oracles::price_feed::market_map::v2::{
        Market,
        ProviderConfig,
        Ticker,
    },
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
            TracePrefixed,
        },
        Address,
        Bech32,
        RollupId,
    },
    protocol::{
        price_feed::v1::ExtendedCommitInfoWithCurrencyPairMapping,
        transaction::v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeTransfer,
            BridgeUnlock,
            CurrencyPairsChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            MarketsChange,
            RecoverIbcClient,
            RollupDataSubmission,
            Transfer,
        },
    },
    sequencerblock::v1::{
        block::Deposit,
        DataItem,
    },
};
use bytes::Bytes;
use ibc_proto::{
    ibc::lightclients::tendermint::v1::ClientState as RawTmClientState,
    ics23::ProofSpec,
};
use ibc_types::{
    core::client::msgs::MsgCreateClient,
    lightclients::tendermint::{
        client_state::{
            AllowUpdate,
            ClientState,
            TENDERMINT_CLIENT_STATE_TYPE_URL,
        },
        consensus_state::TENDERMINT_CONSENSUS_STATE_TYPE_URL,
        TrustThreshold,
    },
};
use penumbra_ibc::IbcRelay;
use prost::Message as _;
use tendermint::block::Height;

pub(crate) use self::{
    bridge_initializer::BridgeInitializer,
    chain_initializer::ChainInitializer,
    checked_tx_builder::CheckedTxBuilder,
    fixture::Fixture,
    ics20_withdrawal_builder::Ics20WithdrawalBuilder,
};
use crate::{
    checked_transaction::CheckedTransaction,
    proposal::commitment::generate_rollup_datas_commitment,
};

mod bridge_initializer;
mod chain_initializer;
mod checked_tx_builder;
mod fixture;
mod ics20_withdrawal_builder;

pub(crate) const ASTRIA_PREFIX: &str = "astria";
pub(crate) const ASTRIA_COMPAT_PREFIX: &str = "astriacompat";
pub(crate) const TEN_QUINTILLION: u128 = 10_u128.pow(19);

/// By default, [`Fixture`] uses `ALICE` as a funded validator.
pub(crate) static ALICE: LazyLock<SigningKey> = LazyLock::new(|| {
    signing_key_from_hex_seed("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
});
pub(crate) static ALICE_ADDRESS_BYTES: LazyLock<[u8; ADDRESS_LENGTH]> =
    LazyLock::new(|| ALICE.address_bytes());
pub(crate) static ALICE_ADDRESS: LazyLock<Address> =
    LazyLock::new(|| astria_address(&*ALICE_ADDRESS_BYTES));

/// By default, [`Fixture`] uses `BOB` as a funded validator.
pub(crate) static BOB: LazyLock<SigningKey> = LazyLock::new(|| {
    signing_key_from_hex_seed("b70fd3b99cab2d98dbd73602deb026b9cdc9bb7b85d35f0bbb81b17c78923dd0")
});
pub(crate) static BOB_ADDRESS_BYTES: LazyLock<[u8; ADDRESS_LENGTH]> =
    LazyLock::new(|| BOB.address_bytes());
pub(crate) static BOB_ADDRESS: LazyLock<Address> =
    LazyLock::new(|| astria_address(&*BOB_ADDRESS_BYTES));

/// By default, [`Fixture`] uses `CAROL` as a funded validator.
pub(crate) static CAROL: LazyLock<SigningKey> = LazyLock::new(|| {
    signing_key_from_hex_seed("0e951afdcbefc420fe6f71b82b0c28c11eb6ee5d95be0886ce9dbf6fa512debc")
});
pub(crate) static CAROL_ADDRESS_BYTES: LazyLock<[u8; ADDRESS_LENGTH]> =
    LazyLock::new(|| CAROL.address_bytes());
pub(crate) static CAROL_ADDRESS: LazyLock<Address> =
    LazyLock::new(|| astria_address(&*CAROL_ADDRESS_BYTES));

/// By default, [`Fixture`] uses `SUDO` as the sudo address, and is a non-funded account. It is also
/// used as the default sudo address and withdrawer address by the [`BridgeInitializer`].
pub(crate) static SUDO: LazyLock<SigningKey> = LazyLock::new(|| {
    signing_key_from_hex_seed("3b2a05a2168952a102dcc07f39b9e385a45b9c2a9b6e3d06acf46fb39fd14019")
});
pub(crate) static SUDO_ADDRESS_BYTES: LazyLock<[u8; ADDRESS_LENGTH]> =
    LazyLock::new(|| SUDO.address_bytes());
pub(crate) static SUDO_ADDRESS: LazyLock<Address> =
    LazyLock::new(|| astria_address(&*SUDO_ADDRESS_BYTES));

/// By default, [`Fixture`] uses `IBC_SUDO` as the IBC sudo address and the only IBC relayer
/// address. It is a non-funded account.
pub(crate) static IBC_SUDO: LazyLock<SigningKey> = LazyLock::new(|| {
    signing_key_from_hex_seed("db4982e01f3eba9e74ac35422fcd49aa2b47c3c535345c7e7da5220fe3a0ce79")
});
pub(crate) static IBC_SUDO_ADDRESS_BYTES: LazyLock<[u8; ADDRESS_LENGTH]> =
    LazyLock::new(|| IBC_SUDO.address_bytes());
pub(crate) static IBC_SUDO_ADDRESS: LazyLock<Address> =
    LazyLock::new(|| astria_address(&*IBC_SUDO_ADDRESS_BYTES));

fn signing_key_from_hex_seed(hex_seed: &str) -> SigningKey {
    let seed = hex::decode(hex_seed).unwrap();
    SigningKey::try_from(seed.as_slice()).unwrap()
}

pub(crate) fn astria_address(bytes: &[u8]) -> Address {
    Address::builder()
        .prefix(ASTRIA_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}

pub(crate) fn astria_compat_address(bytes: &[u8]) -> Address<Bech32> {
    Address::builder()
        .prefix(ASTRIA_COMPAT_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}

pub(crate) fn borsh_then_hex<T: borsh::BorshSerialize>(item: &T) -> String {
    hex::encode(borsh::to_vec(item).unwrap())
}

/// Converts the inputs to the collection of `Bytes` equivalent to those provided in CometBFT
/// requests and responses as the `txs` field.
pub(crate) fn transactions_with_extended_commit_info_and_commitments(
    block_height: Height,
    txs: &[Arc<CheckedTransaction>],
    deposits: Option<HashMap<RollupId, Vec<Deposit>>>,
) -> Vec<Bytes> {
    // If vote extensions are enabled at block height 1 (the minimum possible), then the first
    // block to include extended commit info is at height 2.
    assert!(
        block_height > tendermint::block::Height::from(1_u8),
        "extended commit info can only be applied to block height 2 or greater"
    );

    let extended_commit_info = ExtendedCommitInfoWithCurrencyPairMapping::empty(0u16.into());
    let encoded_extended_commit_info =
        DataItem::ExtendedCommitInfo(extended_commit_info.into_raw().encode_to_vec().into())
            .encode();
    let commitments = generate_rollup_datas_commitment::<true>(txs, deposits.unwrap_or_default());
    let txs_with_commit_info: Vec<Bytes> = commitments
        .into_iter()
        .chain(std::iter::once(encoded_extended_commit_info))
        .chain(txs.iter().map(|tx| tx.encoded_bytes().clone()))
        .collect();
    txs_with_commit_info
}

/// Returns a `BridgeLock` action with the following dummy values:
///   * `to`: `astria_address(&[50; ADDRESS_LENGTH])`
///   * `amount`: 100
///   * `asset`: nria
///   * `fee_asset`: nria
///   * `destination_chain_address`: "test-chain"
pub(crate) fn dummy_bridge_lock() -> BridgeLock {
    BridgeLock {
        to: astria_address(&[50; ADDRESS_LENGTH]),
        amount: 100,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "test-chain".to_string(),
    }
}

/// Returns a `BridgeSudoChange` action with the following dummy values:
///   * `bridge_address`: `astria_address(&[99; ADDRESS_LENGTH])`
///   * `new_sudo_address`: `Some(astria_address(&[98; ADDRESS_LENGTH]))`
///   * `new_withdrawer_address`: `Some(astria_address(&[97; ADDRESS_LENGTH]))`
///   * `fee_asset`: `"test".parse()`
pub(crate) fn dummy_bridge_sudo_change() -> BridgeSudoChange {
    BridgeSudoChange {
        bridge_address: astria_address(&[99; ADDRESS_LENGTH]),
        new_sudo_address: Some(astria_address(&[98; ADDRESS_LENGTH])),
        new_withdrawer_address: Some(astria_address(&[97; ADDRESS_LENGTH])),
        fee_asset: "test".parse().unwrap(),
    }
}

/// Returns a `BridgeTransfer` action with the following dummy values:
///   * `to`: `astria_address(&[99; ADDRESS_LENGTH])`
///   * `amount`: 100
///   * `fee_asset`: nria
///   * `destination_chain_address`: "test-chain"
///   * `bridge_address`: `astria_address(&[50; ADDRESS_LENGTH])`
///   * `rollup_block_number`: 10
///   * `rollup_withdrawal_event_id`: "a-rollup-defined-hash"
pub(crate) fn dummy_bridge_transfer() -> BridgeTransfer {
    BridgeTransfer {
        to: astria_address(&[99; ADDRESS_LENGTH]),
        amount: 100,
        fee_asset: nria().into(),
        destination_chain_address: "test-chain".to_string(),
        bridge_address: astria_address(&[50; ADDRESS_LENGTH]),
        rollup_block_number: 10,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    }
}

/// Returns a `BridgeUnlock` action with the following dummy values:
///   * `to`: `astria_address(&[3; ADDRESS_LENGTH])`
///   * `amount`: 100
///   * `fee_asset`: nria
///   * `memo`: "rollup memo"
///   * `bridge_address`: `astria_address(&[50; ADDRESS_LENGTH])`
///   * `rollup_block_number`: 10
///   * `rollup_withdrawal_event_id`: "a-rollup-defined-hash"
pub(crate) fn dummy_bridge_unlock() -> BridgeUnlock {
    BridgeUnlock {
        to: astria_address(&[3; ADDRESS_LENGTH]),
        amount: 100,
        fee_asset: nria().into(),
        memo: "rollup memo".to_string(),
        bridge_address: astria_address(&[50; ADDRESS_LENGTH]),
        rollup_block_number: 10,
        rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
    }
}

/// Returns a `CurrencyPairsChange::Addition` action with a dummy value of "TIA/USD" and "ETH/USD".
pub(crate) fn dummy_currency_pairs_change() -> CurrencyPairsChange {
    CurrencyPairsChange::Addition(
        ["TIA/USD".parse().unwrap(), "ETH/USD".parse().unwrap()]
            .into_iter()
            .collect(),
    )
}

/// Returns an `IbcRelay::CreateClient` action with dummy values.
pub(crate) fn dummy_ibc_relay() -> IbcRelay {
    use ibc_proto::{
        google::protobuf::{
            Any,
            Timestamp,
        },
        ibc::{
            core::commitment::v1::MerkleRoot,
            lightclients::tendermint::v1::ConsensusState,
        },
    };

    let raw_client_state = RawTmClientState::from(dummy_ibc_client_state(1));
    let raw_consensus_state = ConsensusState {
        timestamp: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        root: Some(MerkleRoot::default()),
        next_validators_hash: vec![],
    };
    IbcRelay::CreateClient(MsgCreateClient {
        client_state: Any {
            type_url: TENDERMINT_CLIENT_STATE_TYPE_URL.to_string(),
            value: raw_client_state.encode_to_vec(),
        },
        consensus_state: Any {
            type_url: TENDERMINT_CONSENSUS_STATE_TYPE_URL.to_string(),
            value: raw_consensus_state.encode_to_vec(),
        },
        signer: String::new(),
    })
}

/// Returns a `ClientState` with dummy values.
pub(crate) fn dummy_ibc_client_state(rev_height: u64) -> ClientState {
    let version = 2;
    let chain_id = ibc_types::core::connection::ChainId::new("test".to_string(), version);
    let proof_spec = ProofSpec {
        leaf_spec: None,
        inner_spec: None,
        max_depth: 0,
        min_depth: 0,
        prehash_key_before_comparison: false,
    };
    let height = ibc_types::core::client::Height::new(version, rev_height).unwrap();
    let allow_update = AllowUpdate {
        after_expiry: true,
        after_misbehaviour: true,
    };
    ClientState::new(
        chain_id,
        TrustThreshold::TWO_THIRDS,
        Duration::from_secs(1),
        Duration::from_secs(64_000),
        Duration::from_secs(1),
        height,
        vec![proof_spec],
        vec![],
        allow_update,
        None,
    )
    .unwrap()
}

/// Returns an `Ics20Withdrawal` action constructed from an [`Ics20WithdrawalBuilder`] using its
/// default values.
pub(crate) fn dummy_ics20_withdrawal() -> Ics20Withdrawal {
    Ics20WithdrawalBuilder::new().build()
}

/// Returns an `InitBridgeAccount` action with the following dummy values:
///   * `rollup_id`: `[1; 32]`
///   * `asset`: `"test".parse()`
///   * `fee_asset`: `"test".parse()`
///   * `sudo_address`: `Some(astria_address(&[2; ADDRESS_LENGTH]))`
///   * `withdrawer_address`: `Some(astria_address(&[3; ADDRESS_LENGTH]))`
pub(crate) fn dummy_init_bridge_account() -> InitBridgeAccount {
    InitBridgeAccount {
        rollup_id: RollupId::new([1; 32]),
        asset: "test".parse().unwrap(),
        fee_asset: "test".parse().unwrap(),
        sudo_address: Some(astria_address(&[2; ADDRESS_LENGTH])),
        withdrawer_address: Some(astria_address(&[3; ADDRESS_LENGTH])),
    }
}

/// Returns a `MarketsChange::Creation` action with the following dummy value as the single `Market`
/// entry:
///    * `ticker`:
///      * `currency_pair`: "TIA/USD"
///      * `decimals`: 9
///      * `min_provider_count`: 2
///      * `enabled`: true
///      * `metadata_json`: "dummy ticker"
///    * `provider_configs` (one entry as follows):
///      * `name`: `coingecko_api`
///      * `off_chain_ticker`: "celestia/usd"
///      * `normalize_by_pair`: `None`
///      * `invert`: false
///      * `metadata_json`: "dummy provider"
pub(crate) fn dummy_markets_change() -> MarketsChange {
    MarketsChange::Creation(vec![Market {
        ticker: dummy_ticker("TIA/USD", "dummy ticker"),
        provider_configs: vec![ProviderConfig {
            name: "coingecko_api".to_string(),
            off_chain_ticker: "celestia/usd".to_string(),
            normalize_by_pair: None,
            invert: false,
            metadata_json: "dummy provider".to_string(),
        }],
    }])
}

/// Returns a `RecoverIbcClient` action with the following dummy values:
///   * `client_id`: "test-id", 0
///   * `replacement_client_id`: "test-id", 1
pub(crate) fn dummy_recover_ibc_client() -> RecoverIbcClient {
    use ibc_types::core::client::{
        ClientId,
        ClientType,
    };

    RecoverIbcClient {
        client_id: ClientId::new(ClientType::new("test-id".to_string()), 0).unwrap(),
        replacement_client_id: ClientId::new(ClientType::new("test-id".to_string()), 1).unwrap(),
    }
}

/// Returns a `RollupDataSubmission` action with the following dummy values:
///   * `rollup_id`: `[1; 32]`
///   * `data`: `[1, 2, 3]`
///   * `fee_asset`: nria
pub(crate) fn dummy_rollup_data_submission() -> RollupDataSubmission {
    RollupDataSubmission {
        rollup_id: RollupId::new([1; 32]),
        data: Bytes::from(vec![1, 2, 3]),
        fee_asset: nria().into(),
    }
}

/// Returns a `Ticker` with the following dummy values:
///   * `decimals`: 9
///   * `min_provider_count`: 2
///   * `enabled`: true
pub(crate) fn dummy_ticker(currency_pair: &str, metadata: &str) -> Ticker {
    Ticker {
        currency_pair: currency_pair.parse().unwrap(),
        decimals: 9,
        min_provider_count: 2,
        enabled: true,
        metadata_json: metadata.to_string(),
    }
}

/// Returns a `Transfer` action with the following dummy values:
///   * `to`: `astria_address(&[50; ADDRESS_LENGTH])`
///   * `fee_asset`: nria
///   * `asset`: nria
///   * `amount`: 100
pub(crate) fn dummy_transfer() -> Transfer {
    Transfer {
        to: astria_address(&[50; ADDRESS_LENGTH]),
        fee_asset: nria().into(),
        asset: nria().into(),
        amount: 100,
    }
}

pub(crate) fn nria() -> TracePrefixed {
    "nria".parse().unwrap()
}

pub(crate) fn denom_0() -> Denom {
    nria().into()
}

pub(crate) fn denom_1() -> Denom {
    "denom_1".parse().unwrap()
}

pub(crate) fn denom_2() -> Denom {
    "denom_2".parse().unwrap()
}

pub(crate) fn denom_3() -> Denom {
    "denom_3".parse().unwrap()
}

pub(crate) fn denom_4() -> Denom {
    "denom_4".parse().unwrap()
}

pub(crate) fn denom_5() -> Denom {
    "denom_5".parse().unwrap()
}

pub(crate) fn denom_6() -> Denom {
    "denom_6".parse().unwrap()
}

pub(crate) fn dummy_tx_costs(
    denom_0_cost: u128,
    denom_1_cost: u128,
    denom_2_cost: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut costs: HashMap<IbcPrefixed, u128> = HashMap::<IbcPrefixed, u128>::new();
    costs.insert(denom_0().to_ibc_prefixed(), denom_0_cost);
    costs.insert(denom_1().to_ibc_prefixed(), denom_1_cost);
    costs.insert(denom_2().to_ibc_prefixed(), denom_2_cost); // not present in balances

    // we don't sanitize the cost inputs
    costs.insert(denom_5().to_ibc_prefixed(), 0); // zero in balances also
    costs.insert(denom_6().to_ibc_prefixed(), 0); // not present in balances

    costs
}

pub(crate) fn dummy_balances(
    denom_0_balance: u128,
    denom_1_balance: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut balances = HashMap::<IbcPrefixed, u128>::new();
    if denom_0_balance != 0 {
        balances.insert(denom_0().to_ibc_prefixed(), denom_0_balance);
    }
    if denom_1_balance != 0 {
        balances.insert(denom_1().to_ibc_prefixed(), denom_1_balance);
    }
    // we don't sanitize the balance inputs
    balances.insert(denom_3().to_ibc_prefixed(), 100); // balance transaction costs won't have entry for
    balances.insert(denom_4().to_ibc_prefixed(), 0); // zero balance not in transaction
    balances.insert(denom_5().to_ibc_prefixed(), 0); // zero balance with corresponding zero cost

    balances
}

#[track_caller]
pub(crate) fn assert_error_contains<T: Debug>(error: &T, expected: &'_ str) {
    let msg = format!("{error:?}");
    assert!(
        msg.contains(expected),
        "error contained different message\n\texpected: {expected}\n\tfull_error: {msg}",
    );
}
