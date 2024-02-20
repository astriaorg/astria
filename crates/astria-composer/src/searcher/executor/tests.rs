use std::time::Duration;

use astria_core::sequencer::v1alpha1::{
    asset::default_native_asset_id,
    transaction::action::SequenceAction,
    RollupId,
    ROLLUP_ID_LEN,
};
use color_eyre::eyre;
use once_cell::sync::Lazy;
use prost::Message;
use sequencer_client::SignedTransaction;
use serde_json::json;
use tendermint_rpc::{
    endpoint::broadcast::tx_sync,
    request,
    response,
    Id,
};
use tokio::{
    sync::{
        mpsc,
        watch,
    },
    time,
};
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
    searcher::executor::{
        Executor,
        Status,
    },
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

/// Start a mock sequencer server and mount a mock for the `accounts/nonce` query.
async fn setup() -> (MockServer, MockGuard, Config) {
    use astria_core::generated::sequencer::v1alpha1::NonceResponse;
    Lazy::force(&TELEMETRY);
    let server = MockServer::start().await;
    let startup_guard = mount_nonce_query_mock(
        &server,
        "accounts/nonce",
        NonceResponse {
            height: 0,
            nonce: 0,
        },
    )
    .await;

    let cfg = Config {
        log: String::new(),
        api_listen_addr: "127.0.0.1:0".parse().unwrap(),
        rollups: String::new(),
        sequencer_url: server.uri(),
        private_key: "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90"
            .to_string()
            .into(),
        block_time_ms: 2000,
        max_bytes_per_bundle: 1000,
        no_otel: false,
        force_stdout: false,
        no_metrics: false,
        metrics_http_listener_addr: String::new(),
        pretty_print: true,
    };
    (server, startup_guard, cfg)
}

/// Mount a mock for the `abci_query` endpoint.
async fn mount_nonce_query_mock(
    server: &MockServer,
    query_path: &str,
    response: impl Message,
) -> MockGuard {
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
    use astria_core::generated::sequencer::v1alpha1::SignedTransaction as RawSignedTransaction;
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

/// Deserizalizes the bytes contained in a `tx_sync::Request` to a signed sequencer transaction
/// and verifies that the contained sequence action is in the given `expected_chain_ids` and
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

/// Helper to wait for the executor to connect to the mock sequencer
async fn wait_for_startup(
    mut status: watch::Receiver<Status>,
    nonce_guard: MockGuard,
) -> eyre::Result<()> {
    // wait to receive executor status
    status.wait_for(Status::is_connected).await.unwrap();

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
    let (sequencer, nonce_guard, cfg) = setup().await;
    let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
    let executor = Executor::new(
        &cfg.sequencer_url,
        &cfg.private_key,
        seq_actions_rx,
        cfg.block_time_ms,
        cfg.max_bytes_per_bundle,
    )
    .unwrap();
    let status = executor.subscribe();
    let _executor_task = tokio::spawn(executor.run_until_stopped());

    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, the first of which is large enough to fill the
    // bundle sending the second should cause the first to immediately be submitted in
    // order to make space for the second
    let seq0 = SequenceAction {
        rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
        data: vec![0u8; cfg.max_bytes_per_bundle - ROLLUP_ID_LEN],
        fee_asset_id: default_native_asset_id(),
    };

    let seq1 = SequenceAction {
        rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
        data: vec![1u8; 1],
        fee_asset_id: default_native_asset_id(),
    };

    // push both sequence actions to the executor in order to force the full bundle to be sent
    seq_actions_tx.send(seq0.clone()).await.unwrap();
    seq_actions_tx.send(seq1.clone()).await.unwrap();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        response_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    // i.e. only the full bundle was sent and not the second one due to the block timer
    let expected_seq_actions = vec![seq0];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(
        actions.len(),
        expected_seq_actions.len(),
        "received more than one action, one was supposed to fill the bundle"
    );

    for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
        let seq_action = action.as_sequence().unwrap();
        assert_eq!(
            seq_action.rollup_id, expected_seq_action.rollup_id,
            "chain id does not match. actual {:?} expected {:?}",
            seq_action.rollup_id, expected_seq_action.rollup_id
        );
        assert_eq!(
            seq_action.data, expected_seq_action.data,
            "data does not match expected data for action with rollup_id {:?}",
            seq_action.rollup_id,
        );
    }
}

/// Test to check that the executor sends a signed transaction to the sequencer after its
/// `block_timer` has ticked
#[tokio::test]
async fn bundle_triggered_by_block_timer() {
    // set up the executor, channel for writing seq actions, and the sequencer mock
    let (sequencer, nonce_guard, cfg) = setup().await;
    let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
    let executor = Executor::new(
        &cfg.sequencer_url,
        &cfg.private_key,
        seq_actions_rx,
        cfg.block_time_ms,
        cfg.max_bytes_per_bundle,
    )
    .unwrap();
    let status = executor.subscribe();
    let _executor_task = tokio::spawn(executor.run_until_stopped());

    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let seq0 = SequenceAction {
        rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
        data: vec![0u8; cfg.max_bytes_per_bundle / 4],
        fee_asset_id: default_native_asset_id(),
    };

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    seq_actions_tx.send(seq0.clone()).await.unwrap();
    time::advance(Duration::from_millis(cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        response_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_seq_actions = vec![seq0];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(
        actions.len(),
        expected_seq_actions.len(),
        "received more than one action, one was supposed to fill the bundle"
    );

    for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
        let seq_action = action.as_sequence().unwrap();
        assert_eq!(
            seq_action.rollup_id, expected_seq_action.rollup_id,
            "chain id does not match. actual {:?} expected {:?}",
            seq_action.rollup_id, expected_seq_action.rollup_id
        );
        assert_eq!(
            seq_action.data, expected_seq_action.data,
            "data does not match expected data for action with rollup_id {:?}",
            seq_action.rollup_id,
        );
    }
}

/// Test to check that the executor sends a signed transaction with two sequence actions to the
/// sequencer.
#[tokio::test]
async fn two_seq_actions_single_bundle() {
    // set up the executor, channel for writing seq actions, and the sequencer mock
    let (sequencer, nonce_guard, cfg) = setup().await;
    let (seq_actions_tx, seq_actions_rx) = mpsc::channel(2);
    let executor = Executor::new(
        &cfg.sequencer_url,
        &cfg.private_key,
        seq_actions_rx,
        cfg.block_time_ms,
        cfg.max_bytes_per_bundle,
    )
    .unwrap();
    let status = executor.subscribe();
    let _executor_task = tokio::spawn(executor.run_until_stopped());

    // wait for sequencer to get the initial nonce request from sequencer
    wait_for_startup(status, nonce_guard).await.unwrap();

    let response_guard = mount_broadcast_tx_sync_seq_actions_mock(&sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let seq0 = SequenceAction {
        rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
        data: vec![0u8; cfg.max_bytes_per_bundle / 4],
        fee_asset_id: default_native_asset_id(),
    };

    let seq1 = SequenceAction {
        rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
        data: vec![1u8; cfg.max_bytes_per_bundle / 4],
        fee_asset_id: default_native_asset_id(),
    };

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    seq_actions_tx.send(seq0.clone()).await.unwrap();
    seq_actions_tx.send(seq1.clone()).await.unwrap();
    time::advance(Duration::from_millis(cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        response_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_seq_actions = vec![seq0, seq1];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(
        actions.len(),
        expected_seq_actions.len(),
        "received more than one action, one was supposed to fill the bundle"
    );

    for (action, expected_seq_action) in actions.iter().zip(expected_seq_actions.iter()) {
        let seq_action = action.as_sequence().unwrap();
        assert_eq!(
            seq_action.rollup_id, expected_seq_action.rollup_id,
            "chain id does not match. actual {:?} expected {:?}",
            seq_action.rollup_id, expected_seq_action.rollup_id
        );
        assert_eq!(
            seq_action.data, expected_seq_action.data,
            "data does not match expected data for action with rollup_id {:?}",
            seq_action.rollup_id,
        );
    }
}
