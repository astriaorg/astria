use std::time::Duration;

use ethers::types::Transaction;
use proto::generated::sequencer::v1alpha1::NonceResponse;
use sequencer_types::AbciCode;
use tendermint_rpc::{
    endpoint::broadcast::tx_sync,
    request::{self,},
    response,
    Id,
};
use tracing::debug;
use wiremock::{
    Mock,
    MockGuard,
    MockServer,
    Request,
    ResponseTemplate,
};

use crate::helper::spawn_composer;

#[tokio::test]
async fn tx_from_one_rollup_is_received_by_sequencer() {
    let test_composer = spawn_composer(&["test1"]).await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("setup guard failed");

    let mock_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, vec!["test1"], vec![0]).await;
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

#[tokio::test]
async fn tx_from_two_rollups_are_received_by_sequencer() {
    let test_composer = spawn_composer(&["test1", "test2"]).await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("setup guard failed");

    let test_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, vec!["test1", "test2"], vec![0, 1])
            .await;
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();
    test_composer.rollup_nodes["test2"]
        .push_tx(Transaction::default())
        .unwrap();

    tokio::time::timeout(
        Duration::from_millis(100),
        test_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast messages from composer");

    // Validate that the received nonces and chain_ids were unique
    let mut received_nonces: Vec<u32> = vec![];
    let mut received_chain_ids: Vec<Vec<u8>> = vec![];
    for request in test_guard.received_requests().await {
        let (chain_id, nonce) = chain_id_nonce_from_request(&request);
        assert!(
            !received_nonces.contains(&nonce),
            "duplicate nonce received"
        );
        received_nonces.push(nonce);

        assert!(
            !received_chain_ids.contains(&chain_id),
            "duplicate chain id received"
        );
        received_chain_ids.push(chain_id);
    }
}

#[tokio::test]
async fn invalid_nonce_failure_causes_tx_resubmission_under_different_nonce() {
    use crate::helper::mock_sequencer::mount_abci_query_mock;

    // Spawn a composer with a mock sequencer and a mock rollup node
    // Initial nonce is 0
    let test_composer = spawn_composer(&["test1"]).await;
    tokio::time::timeout(
        Duration::from_millis(100),
        test_composer.setup_guard.wait_until_satisfied(),
    )
    .await
    .expect("setup guard failed");

    // Reject the first transaction for invalid nonce
    let invalid_nonce_guard =
        mount_broadcast_tx_sync_invalid_nonce_mock(&test_composer.sequencer, "test1").await;

    // Mount a response of 0 to a nonce query
    let nonce_refetch_guard = mount_abci_query_mock(
        &test_composer.sequencer,
        "accounts/nonce",
        NonceResponse {
            height: 0,
            nonce: 1,
        },
    )
    .await;

    // Expect nonce 1 again so that the resubmitted tx is accepted
    let valid_nonce_guard =
        mount_broadcast_tx_sync_mock(&test_composer.sequencer, vec!["test1"], vec![1]).await;

    // Push a tx to the rollup node so that it is picked up by the composer and submitted with the
    // stored nonce of 0, triggering the nonce refetch process
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();

    tokio::time::timeout(
        Duration::from_millis(100),
        invalid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("invalid nonce guard failed");

    tokio::time::timeout(
        Duration::from_millis(100),
        nonce_refetch_guard.wait_until_satisfied(),
    )
    .await
    .expect("nonce refetch guard failed");

    tokio::time::timeout(
        Duration::from_millis(100),
        valid_nonce_guard.wait_until_satisfied(),
    )
    .await
    .expect("valid nonce guard failed");
}

/// Deserizalizes the bytes contained in a `tx_sync::Request` to a signed sequencer transaction and
/// verifies that the contained sequence action is in the given `expected_chain_ids` and
/// `expected_nonces`.
async fn mount_broadcast_tx_sync_mock(
    server: &MockServer,
    expected_chain_ids: Vec<&'static str>,
    expected_nonces: Vec<u32>,
) -> MockGuard {
    let expected_calls = expected_nonces.len().try_into().unwrap();
    let matcher = move |request: &Request| {
        let (chain_id, nonce) = chain_id_nonce_from_request(request);

        let valid_chain_id = expected_chain_ids.contains(&std::str::from_utf8(&chain_id).unwrap());
        let valid_nonce = expected_nonces.contains(&nonce);

        valid_chain_id && valid_nonce
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
        .up_to_n_times(expected_calls)
        .expect(expected_calls)
        .mount_as_scoped(server)
        .await
}

/// Deserizalizes the bytes contained in a `tx_sync::Request` to a signed sequencer transaction and
/// verifies that the contained sequence action is for the given `expected_chain_id`. It then
/// rejects the transaction for an invalid nonce.
async fn mount_broadcast_tx_sync_invalid_nonce_mock(
    server: &MockServer,
    expected_chain_id: &'static str,
) -> MockGuard {
    let matcher = move |request: &Request| {
        let (chain_id, _) = chain_id_nonce_from_request(request);
        chain_id == expected_chain_id.as_bytes()
    };
    let jsonrpc_rsp = response::Wrapper::new_with_id(
        Id::Num(1),
        Some(tx_sync::Response {
            code: AbciCode::INVALID_NONCE.into(),
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

fn chain_id_nonce_from_request(request: &Request) -> (Vec<u8>, u32) {
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        native::sequencer::v1alpha1::SignedTransaction,
        Message as _,
    };

    let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
        serde_json::from_slice(&request.body)
            .expect("can't deserialize to JSONRPC wrapped tx_sync::Request");
    let raw_signed_tx = raw::SignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
        .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
        .expect("can't convert raw signed tx to checked signed tx");
    debug!(?signed_tx, "sequencer mock received signed transaction");
    let Some(sent_action) = signed_tx.actions().get(0) else {
        panic!("received transaction contained no actions");
    };
    let Some(sequence_action) = sent_action.as_sequence() else {
        panic!("mocked sequencer expected a sequence action");
    };

    (
        sequence_action.chain_id.clone(),
        signed_tx.unsigned_transaction().nonce,
    )
}
