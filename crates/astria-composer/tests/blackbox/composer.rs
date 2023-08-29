use std::time::Duration;

use ethers::types::Transaction;
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
    let mock_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test1").await;
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
    use futures::future::join;

    let test_composer = spawn_composer(&["test1", "test2"]).await;
    let test1_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test1").await;
    let test2_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test2").await;
    test_composer.rollup_nodes["test1"]
        .push_tx(Transaction::default())
        .unwrap();
    test_composer.rollup_nodes["test2"]
        .push_tx(Transaction::default())
        .unwrap();
    let all_guards = join(
        test1_guard.wait_until_satisfied(),
        test2_guard.wait_until_satisfied(),
    );
    tokio::time::timeout(Duration::from_millis(100), all_guards)
        .await
        .expect("mocked sequencer should have received a broadcast message from composer");
}

/// Deserizalizes the bytes contained in a `tx_sync::Request` to a
/// signed sequencer transaction and verifies that the contained
/// sequence action is for the given `expected_chain_id`.
async fn mount_broadcast_tx_sync_mock(
    server: &MockServer,
    expected_chain_name: &'static str,
) -> MockGuard {
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        native::sequencer::v1alpha1::{
            ChainId,
            SignedTransaction,
        },
        Message as _,
    };
    let matcher = move |request: &Request| {
        let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
            serde_json::from_slice(&request.body)
                .expect("can't deserialize to JSONRPC wrapped tx_sync::Request");
        let raw_signed_tx = raw::SignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
            .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
        let signed_tx = SignedTransaction::try_from_proto(raw_signed_tx)
            .expect("can't convert raw signed tx to checked signed tx");
        debug!(?signed_tx, "sequencer mock received signed transaction");
        let Some(sent_action) = signed_tx.actions().get(0) else {
            panic!("received transaction contained no actions");
        };
        let Some(sequence_action) = sent_action.as_sequence() else {
            panic!("mocked sequencer expected a sequence action");
        };
        let expected_chain_id = ChainId::with_hashed_bytes(expected_chain_name);
        sequence_action.chain_id == expected_chain_id
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
        .expect(1)
        .mount_as_scoped(server)
        .await
}
