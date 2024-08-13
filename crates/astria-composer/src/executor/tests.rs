use std::{
    io::Write,
    time::Duration,
};

use astria_core::{
    generated::{
        composer::v1alpha1::BuilderBundlePacket,
        protocol::account::v1alpha1::NonceResponse,
        sequencerblock::v1alpha1 as raw_sequencer,
    },
    primitive::v1::{
        asset,
        RollupId,
        ROLLUP_ID_LEN,
    },
    protocol::transaction::v1alpha1::action::SequenceAction,
    sequencerblock::v1alpha1::block::RollupData,
    Protobuf,
};
use astria_eyre::eyre;
use base64::{
    prelude::BASE64_STANDARD,
    Engine,
};
use futures::future::join;
use once_cell::sync::Lazy;
use prost::{
    bytes::Bytes,
    Message as _,
};
use sequencer_client::SignedTransaction;
use serde_json::json;
use tempfile::NamedTempFile;
use tendermint::{
    consensus::{
        params::{
            AbciParams,
            ValidatorParams,
        },
        Params,
    },
    Genesis,
    Time,
};
use tendermint_rpc::{
    endpoint::broadcast::tx_sync,
    request,
    response,
    Id,
};
use tokio::{
    sync::watch,
    time,
};
use tokio_util::sync::CancellationToken;
use tracing::debug;
use wiremock::{
    matchers::{
        body_partial_json,
        body_string_contains,
    },
    Mock,
    MockGuard,
    MockServer,
    Request,
    ResponseTemplate,
};

use crate::{
    executor,
    executor::EnsureChainIdError,
    metrics::Metrics,
    mock_grpc::{
        MockGrpc,
        TestExecutor,
    },
    mount_executed_block,
    mount_get_commitment_state,
    test_utils::sequence_action_of_max_size,
    Config,
};

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .filter_directives(&filter_directives)
            .try_init()
            .unwrap();
    } else {
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::sink)
            .try_init()
            .unwrap();
    }
});

fn sequence_action(rollup_id: RollupId, fee_asset: asset::Denom) -> SequenceAction {
    SequenceAction {
        rollup_id,
        data: Bytes::from(vec![]),
        fee_asset,
    }
}

/// Start a mock sequencer server and mount a mock for the `accounts/nonce` query.
async fn setup() -> (MockServer, Config, NamedTempFile, TestExecutor) {
    Lazy::force(&TELEMETRY);
    let server = MockServer::start().await;
    let execution_api_server = MockGrpc::spawn().await;

    let keyfile = NamedTempFile::new().unwrap();
    (&keyfile)
        .write_all("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes())
        .unwrap();

    let cfg = Config {
        log: String::new(),
        api_listen_addr: "127.0.0.1:0".parse().unwrap(),
        rollup: "test-chain-1".to_string(),
        rollup_websocket_url: String::new(),
        sequencer_url: server.uri(),
        sequencer_chain_id: "test-chain-1".to_string(),
        private_key_file: keyfile.path().to_string_lossy().to_string(),
        sequencer_address_prefix: "astria".into(),
        block_time_ms: 2000,
        max_bytes_per_bundle: 1000,
        bundle_queue_capacity: 10,
        no_otel: false,
        force_stdout: false,
        no_metrics: false,
        metrics_http_listener_addr: String::new(),
        pretty_print: true,
        grpc_addr: "127.0.0.1:0".parse().unwrap(),
        fee_asset: "nria"
            .parse::<asset::Denom>()
            .unwrap()
            .to_ibc_prefixed()
            .into(),
        execution_api_url: format!("http://{}", execution_api_server.local_addr),
    };
    (
        server,
        cfg,
        keyfile,
        TestExecutor {
            mock_grpc: execution_api_server,
        },
    )
}

/// Assert that given error is of correct type and contains the expected chain IDs.
#[track_caller]
fn assert_chain_id_err(
    err: &EnsureChainIdError,
    configured_expected: &str,
    configured_actual: &tendermint::chain::Id,
) {
    match err {
        EnsureChainIdError::WrongChainId {
            expected,
            actual,
        } => {
            assert_eq!(*expected, configured_expected);
            assert_eq!(*actual, *configured_actual);
        }
        other @ EnsureChainIdError::GetChainId(_) => {
            panic!("expected `EnsureChainIdError::WrongChainId`, but got '{other:?}'")
        }
    }
}

/// Mount a mock for the `abci_query` endpoint.
async fn mount_default_nonce_query_mock(server: &MockServer) -> MockGuard {
    let query_path = "accounts/nonce";
    let response = NonceResponse {
        height: 0,
        nonce: 0,
    };
    let expected_body = json!({
        "method": "abci_query"
    });
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: response.encode_to_vec(),
            ..Default::default()
        },
    };
    let wrapper = response::Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(&expected_body))
        .and(body_string_contains(query_path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&wrapper)
                .append_header("Content-Type", "application/json"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount_as_scoped(server)
        .await
}

/// Convert a `Request` object to a `SignedTransaction`
fn signed_tx_from_request(request: &Request) -> SignedTransaction {
    use astria_core::generated::protocol::transaction::v1alpha1::SignedTransaction as RawSignedTransaction;
    use prost::Message as _;

    let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
        serde_json::from_slice(&request.body)
            .expect("can't deserialize to JSONRPC wrapped tx_sync::Request");
    let raw_signed_tx = RawSignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
        .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
        .expect("can't convert raw signed tx to checked signed tx");
    debug!(?signed_tx, "sequencer mock received signed transaction");

    signed_tx
}

/// Deserializes the bytes contained in a `tx_sync::Request` to a signed sequencer transaction
/// and verifies that the contained sequence action is in the given `expected_rollup_ids` and
/// `expected_nonces`.
async fn mount_broadcast_tx_sync_seq_actions_mock(server: &MockServer) -> MockGuard {
    let matcher = move |request: &Request| {
        let signed_tx = signed_tx_from_request(request);
        let actions = signed_tx.actions();

        // verify all received actions are sequence actions
        actions.iter().all(|action| action.as_sequence().is_some())
    };
    let jsonrpc_rsp = response::Wrapper::new_with_id(
        Id::Num(1),
        Some(tx_sync::Response {
            code: 0.into(),
            data: vec![].into(),
            log: String::new(),
            hash: tendermint::Hash::Sha256([0; 32]),
        }),
        None,
    );

    Mock::given(matcher)
        .respond_with(ResponseTemplate::new(200).set_body_json(&jsonrpc_rsp))
        .up_to_n_times(1)
        .expect(1)
        .mount_as_scoped(server)
        .await
}

/// Mounts genesis file with specified sequencer chain ID
async fn mount_genesis(server: &MockServer, mock_sequencer_chain_id: &str) {
    Mock::given(body_partial_json(
        json!({"jsonrpc": "2.0", "method": "genesis", "params": null}),
    ))
    .respond_with(ResponseTemplate::new(200).set_body_json(
        tendermint_rpc::response::Wrapper::new_with_id(
            tendermint_rpc::Id::uuid_v4(),
            Some(
                tendermint_rpc::endpoint::genesis::Response::<serde_json::Value> {
                    genesis: Genesis {
                        genesis_time: Time::from_unix_timestamp(1, 1).unwrap(),
                        chain_id: mock_sequencer_chain_id.try_into().unwrap(),
                        initial_height: 1,
                        consensus_params: Params {
                            block: tendermint::block::Size {
                                max_bytes: 1024,
                                max_gas: 1024,
                                time_iota_ms: 1000,
                            },
                            evidence: tendermint::evidence::Params {
                                max_age_num_blocks: 1000,
                                max_age_duration: tendermint::evidence::Duration(
                                    Duration::from_secs(3600),
                                ),
                                max_bytes: 1_048_576,
                            },
                            validator: ValidatorParams {
                                pub_key_types: vec![tendermint::public_key::Algorithm::Ed25519],
                            },
                            version: None,
                            abci: AbciParams::default(),
                        },
                        validators: vec![],
                        app_hash: tendermint::hash::AppHash::default(),
                        app_state: serde_json::Value::Null,
                    },
                },
            ),
            None,
        ),
    ))
    .expect(1..)
    .mount(server)
    .await;
}

/// Helper to wait for the executor to connect to the mock sequencer
async fn wait_for_startup(
    mut status: watch::Receiver<executor::Status>,
    nonce_guard: MockGuard,
) -> eyre::Result<()> {
    // wait to receive executor status
    status
        .wait_for(executor::Status::is_connected)
        .await
        .unwrap();

    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    Ok(())
}

/// Test to check that the executor sends a signed transaction to the sequencer as soon as it
/// receives a `SequenceAction` that fills it beyond its `max_bundle_size`.
#[tokio::test]
async fn full_bundle() {
    // set up the executor, channel for writing seq actions, and the sequencer mock
    let (sequencer, cfg, _keyfile, test_executor) = setup().await;
    let shutdown_token = CancellationToken::new();
    let metrics = Box::leak(Box::new(Metrics::new(cfg.rollup.clone())));
    mount_genesis(&sequencer, &cfg.sequencer_chain_id).await;
    let (executor, executor_handle) = executor::Builder {
        sequencer_url: cfg.sequencer_url.clone(),
        sequencer_chain_id: cfg.sequencer_chain_id.clone(),
        private_key_file: cfg.private_key_file.clone(),
        sequencer_address_prefix: "astria".into(),
        block_time_ms: cfg.block_time_ms,
        max_bytes_per_bundle: cfg.max_bytes_per_bundle,
        bundle_queue_capacity: cfg.bundle_queue_capacity,
        shutdown_token: shutdown_token.clone(),
        execution_api_url: cfg.execution_api_url,
        chain_name: cfg.rollup.clone(),
        fee_asset: cfg.fee_asset,
        metrics,
    }
    .build()
    .unwrap();

    let rollup_id = RollupId::from_unhashed_bytes(cfg.rollup.clone());

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let nonce_guard = mount_default_nonce_query_mock(&sequencer).await;
    let status = executor.subscribe();

    let _executor_task = tokio::spawn(executor.run_until_stopped());
    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, the first of which is large enough to fill the
    // bundle sending the second should cause the first to immediately be submitted in
    // order to make space for the second
    let seq0 = SequenceAction {
        rollup_id,
        ..sequence_action_of_max_size(cfg.max_bytes_per_bundle)
    };

    let seq1 = SequenceAction {
        rollup_id,
        ..sequence_action_of_max_size(cfg.max_bytes_per_bundle)
    };

    let rollup_data: Vec<raw_sequencer::RollupData> = vec![seq0.clone()]
        .iter()
        .map(|item| RollupData::SequencedData(item.clone().data).to_raw())
        .collect();

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    // push both sequence actions to the executor in order to force the full bundle to be sent
    executor_handle
        .send_timeout(seq0.clone(), Duration::from_millis(1000))
        .await
        .unwrap();
    executor_handle
        .send_timeout(seq1.clone(), Duration::from_millis(1000))
        .await
        .unwrap();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        join(
            response_guard.wait_until_satisfied(),
            execute_block.wait_until_satisfied(),
        ),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    // i.e. only the full bundle was sent and not the second one due to the block timer
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    // we send only 1 action to the sequencer which is a BuilderBundlePacket
    // we first verify that the action sent to the sequencer is a builder bundle packet.

    // only 1 sequence action which is a BuilderBundlePacket is sent
    assert_eq!(actions.len(), 1);

    // decode the sequence action to its BuilderBundlePacket
    let mut seq_action = actions.iter().next().unwrap().as_sequence().unwrap();
    let proto_builder_bundle_packet =
        BuilderBundlePacket::decode(&mut seq_action.data.clone()).unwrap();
    let builder_bundle_packet = astria_core::composer::v1alpha1::BuilderBundlePacket::try_from_raw(
        proto_builder_bundle_packet.clone(),
    )
    .unwrap();

    assert_eq!(
        builder_bundle_packet.bundle().parent_hash(),
        soft_block_hash.to_vec()
    );

    let bundle_txs = builder_bundle_packet.bundle().transactions();

    // there should only be 1 sequence action in the bundle
    assert_eq!(bundle_txs.len(), 1);

    assert_eq!(seq_action.fee_asset, seq0.fee_asset);
    assert_eq!(seq_action.rollup_id, seq0.rollup_id);

    match bundle_txs.iter().next().unwrap() {
        RollupData::SequencedData(data) => {
            assert_eq!(data.clone(), seq0.data)
        }
        _ => {
            assert!(
                true,
                "expected RollupData::SequencedData, but got {:?}",
                bundle_txs.iter().next().unwrap()
            )
        }
    }
}

/// Test to check that the executor sends a signed transaction to the sequencer after its
/// `block_timer` has ticked
#[tokio::test]
async fn bundle_triggered_by_block_timer() {
    // set up the executor, channel for writing seq actions, and the sequencer mock
    let (sequencer, cfg, _keyfile, test_executor) = setup().await;
    let shutdown_token = CancellationToken::new();
    let metrics = Box::leak(Box::new(Metrics::new(cfg.rollup.clone())));
    mount_genesis(&sequencer, &cfg.sequencer_chain_id).await;
    let (executor, executor_handle) = executor::Builder {
        sequencer_url: cfg.sequencer_url.clone(),
        sequencer_chain_id: cfg.sequencer_chain_id.clone(),
        private_key_file: cfg.private_key_file.clone(),
        sequencer_address_prefix: "astria".into(),
        block_time_ms: cfg.block_time_ms,
        max_bytes_per_bundle: cfg.max_bytes_per_bundle,
        bundle_queue_capacity: cfg.bundle_queue_capacity,
        shutdown_token: shutdown_token.clone(),
        execution_api_url: cfg.execution_api_url,
        chain_name: cfg.rollup.clone(),
        fee_asset: cfg.fee_asset.clone(),
        metrics,
    }
    .build()
    .unwrap();

    let rollup_id = RollupId::from_unhashed_bytes(cfg.rollup.clone());

    let nonce_guard = mount_default_nonce_query_mock(&sequencer).await;
    let status = executor.subscribe();

    let _executor_task = tokio::spawn(executor.run_until_stopped());

    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let seq0 = SequenceAction {
        data: Bytes::from(vec![0u8; cfg.max_bytes_per_bundle / 4]),
        ..sequence_action(rollup_id.clone(), cfg.fee_asset.clone())
    };

    let rollup_data: Vec<raw_sequencer::RollupData> = vec![seq0.clone()]
        .iter()
        .map(|item| RollupData::SequencedData(item.clone().data).to_raw())
        .collect();

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    executor_handle
        .send_timeout(seq0.clone(), Duration::from_millis(1000))
        .await
        .unwrap();
    time::advance(Duration::from_millis(cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        join(
            response_guard.wait_until_satisfied(),
            execute_block.wait_until_satisfied(),
        ),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_seq_actions = [seq0];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(actions.len(), 1);

    let mut seq_action = actions.iter().next().unwrap().as_sequence().unwrap();
    let proto_builder_bundle_packet =
        BuilderBundlePacket::decode(&mut seq_action.data.clone()).unwrap();
    let builder_bundle_packet = astria_core::composer::v1alpha1::BuilderBundlePacket::try_from_raw(
        proto_builder_bundle_packet.clone(),
    )
    .unwrap();

    assert_eq!(builder_bundle_packet.bundle().transactions().len(), 1);

    assert_eq!(
        builder_bundle_packet.bundle().parent_hash().to_vec(),
        soft_block_hash.to_vec()
    );

    // ensure that the seq_action of the BuilderBundlePacket and the expected sequence actions have
    // the same rollup id and fee asset

    for (action, expected_action) in expected_seq_actions.iter().zip(actions) {
        let expected_seq_action = expected_action.as_sequence().unwrap();
        assert_eq!(action.rollup_id, expected_seq_action.rollup_id);
        assert_eq!(action.fee_asset, expected_seq_action.fee_asset);
    }

    for (action, expected_seq_action) in builder_bundle_packet
        .bundle()
        .transactions()
        .iter()
        .zip(expected_seq_actions.iter())
    {
        match action.clone() {
            RollupData::SequencedData(data) => {
                assert_eq!(
                    data, expected_seq_action.data,
                    "data does not match expected data for action with rollup_id {:?}",
                    expected_seq_action.rollup_id
                )
            }
            _ => {
                assert!(
                    true,
                    "expected RollupData::SequencedData, but got {:?}",
                    action
                )
            }
        }
    }
}

/// Test to check that the executor sends a signed transaction with two sequence actions to the
/// sequencer.
#[tokio::test]
async fn two_seq_actions_single_bundle() {
    // set up the executor, channel for writing seq actions, and the sequencer mock
    let (sequencer, cfg, _keyfile, test_executor) = setup().await;
    let shutdown_token = CancellationToken::new();
    let metrics = Box::leak(Box::new(Metrics::new(cfg.rollup.clone())));
    mount_genesis(&sequencer, &cfg.sequencer_chain_id).await;
    let (executor, executor_handle) = executor::Builder {
        sequencer_url: cfg.sequencer_url.clone(),
        sequencer_chain_id: cfg.sequencer_chain_id.clone(),
        private_key_file: cfg.private_key_file.clone(),
        sequencer_address_prefix: "astria".into(),
        block_time_ms: cfg.block_time_ms,
        max_bytes_per_bundle: cfg.max_bytes_per_bundle,
        bundle_queue_capacity: cfg.bundle_queue_capacity,
        shutdown_token: shutdown_token.clone(),
        execution_api_url: cfg.execution_api_url,
        chain_name: cfg.rollup.clone(),
        fee_asset: cfg.fee_asset.clone(),
        metrics,
    }
    .build()
    .unwrap();

    let rollup_id = RollupId::from_unhashed_bytes(cfg.rollup.clone());

    let nonce_guard = mount_default_nonce_query_mock(&sequencer).await;
    let status = executor.subscribe();
    let _executor_task = tokio::spawn(executor.run_until_stopped());

    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let seq0 = SequenceAction {
        data: vec![0u8; cfg.max_bytes_per_bundle / 4].into(),
        ..sequence_action(rollup_id.clone(), cfg.fee_asset.clone())
    };

    let seq1 = SequenceAction {
        data: vec![1u8; cfg.max_bytes_per_bundle / 4].into(),
        ..sequence_action(rollup_id.clone(), cfg.fee_asset.clone())
    };

    let rollup_data: Vec<raw_sequencer::RollupData> = vec![seq0.clone(), seq1.clone()]
        .iter()
        .map(|item| RollupData::SequencedData(item.clone().data).to_raw())
        .collect();

    let soft_parent_hash = [1; 64];
    let soft_block_number = 1;
    let soft_block_hash = [2; 64];

    mount_get_commitment_state!(
        test_executor,
        firm: ( number: 1, hash: [1; 64], parent: [0; 64], ),
        soft: ( number: soft_block_number, hash: soft_block_hash, parent: soft_parent_hash, ),
        base_celestia_height: 1,
    );

    let execute_block = mount_executed_block!(test_executor,
        mock_name: "execute_block",
        number: soft_block_number,
        hash: soft_block_hash,
        included_transactions: rollup_data.clone(),
        parent: soft_parent_hash.to_vec(),
    );

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    executor_handle
        .send_timeout(seq0.clone(), Duration::from_millis(1000))
        .await
        .unwrap();
    executor_handle
        .send_timeout(seq1.clone(), Duration::from_millis(1000))
        .await
        .unwrap();
    time::advance(Duration::from_millis(cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        join(
            response_guard.wait_until_satisfied(),
            execute_block.wait_until_satisfied(),
        ),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_seq_actions = [seq0, seq1];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(actions.len(), 1);

    let seq_action = actions.iter().next().unwrap().as_sequence().unwrap();
    let proto_builder_bundle_packet =
        BuilderBundlePacket::decode(&mut seq_action.data.clone()).unwrap();
    let builder_bundle_packet = astria_core::composer::v1alpha1::BuilderBundlePacket::try_from_raw(
        proto_builder_bundle_packet.clone(),
    )
    .unwrap();

    let bundle_txs = builder_bundle_packet.bundle().transactions();

    assert_eq!(builder_bundle_packet.bundle().transactions().len(), 2);
    assert_eq!(
        builder_bundle_packet.bundle().parent_hash().to_vec(),
        soft_block_hash.to_vec()
    );

    for (action, expected_action) in expected_seq_actions.iter().zip(actions) {
        let expected_seq_action = expected_action.as_sequence().unwrap();
        assert_eq!(action.rollup_id, expected_seq_action.rollup_id);
        assert_eq!(action.fee_asset, expected_seq_action.fee_asset);
    }

    for (action, expected_seq_action) in bundle_txs.iter().zip(expected_seq_actions.iter()) {
        match action.clone() {
            RollupData::SequencedData(data) => {
                assert_eq!(
                    data, expected_seq_action.data,
                    "data does not match expected data for action with rollup_id {:?}",
                    expected_seq_action.rollup_id
                )
            }
            _ => assert!(
                true,
                "expected RollupData::SequencedData, but got {:?}",
                action
            ),
        }
    }
}

/// Test to check that executor's chain ID check is properly checked against the sequencer's chain
/// ID
#[tokio::test]
async fn chain_id_mismatch_returns_error() {
    use tendermint::chain::Id;

    // set up sequencer mock
    let (sequencer, cfg, _keyfile, _test_executor) = setup().await;
    let shutdown_token = CancellationToken::new();
    let metrics = Box::leak(Box::new(Metrics::new(cfg.rollup.clone())));
    let rollup_name = RollupId::new([0; ROLLUP_ID_LEN]);

    // mount a status response with an incorrect chain_id
    mount_genesis(&sequencer, "bad-chain-id").await;

    // build the executor with the correct chain_id
    let (executor, _executor_handle) = executor::Builder {
        sequencer_url: cfg.sequencer_url.clone(),
        sequencer_chain_id: cfg.sequencer_chain_id.clone(),
        private_key_file: cfg.private_key_file.clone(),
        sequencer_address_prefix: cfg.sequencer_address_prefix.clone(),
        block_time_ms: cfg.block_time_ms,
        max_bytes_per_bundle: cfg.max_bytes_per_bundle,
        bundle_queue_capacity: cfg.bundle_queue_capacity,
        shutdown_token: shutdown_token.clone(),
        execution_api_url: cfg.execution_api_url,
        chain_name: rollup_name.to_string(),
        fee_asset: cfg.fee_asset,
        metrics,
    }
    .build()
    .unwrap();

    // ensure that run_until_stopped returns WrongChainId error
    let err = executor.run_until_stopped().await.expect_err(
        "should exit with an error when reading a bad chain ID, but exited with success",
    );
    let mut found = false;
    for cause in err.chain() {
        if let Some(err) = cause.downcast_ref::<EnsureChainIdError>() {
            assert_chain_id_err(
                err,
                &cfg.sequencer_chain_id,
                &Id::try_from("bad-chain-id".to_string()).unwrap(),
            );
            found = true;
            break;
        }
    }

    // ensure that the error chain contains the expected error
    assert!(
        found,
        "expected `EnsureChainIdError::WrongChainId` in error chain, but it was not found"
    );
}
