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
    let mock_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test1", 42, None).await;
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
    let test1_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test1", 42, None).await;
    let test2_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test2", 43, None).await;
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

#[tokio::test]
async fn test_tx_integrity() {
    let test_composer = spawn_composer(&["test1"]).await;

    let tx = r#"{
        "hash": "0x077daf1a23be6c48bf5e101b85cc79d9e81969ef901a7099b4fedac3c0d59809",
        "nonce": "0x22e",
        "blockHash": "0xae541fc4dc35d1d8bc2a018160e5ac8876d51ad76539d0b134ac5b82d64e7bda",
        "blockNumber": "0x10fa231",
        "transactionIndex": "0x1",
        "from": "0xe398c02cf1e030b541bdc87efece27ad5ef1e783",
        "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
        "value": "0x0",
        "gasPrice": "0xb2703a824",
        "gas": "0x7a120",
        "input": "0x791ac94700000000000000000000000000000000000000000000000000000a29e1e7c600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000e398c02cf1e030b541bdc87efece27ad5ef1e7830000000000000000000000000000000000000000000000000000000064c5999f00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000ea778a02ab20ce0a8132a0e5312b53a5f23cef5000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        "v": "0x0",
        "r": "0xd768f4d808fc1cb0eedca99363b78d9fa42555b4f26cbf5fa48ba8af96bff159",
        "s": "0x7f4cd55d6d06422ce14f58e72b0f366b479f606d129e4fc959a5eb348c93e888",
        "type": "0x2",
        "accessList": [],
        "maxPriorityFeePerGas": "0x55ae82600",
        "maxFeePerGas": "0x174876e800",
        "chainId": "0x1"
    },"#;

    let tx: Transaction = serde_json::from_str(tx).unwrap();
    let mock_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer, "test1", 42, Some(tx)).await;

    test_composer.rollup_nodes["test1"]
        .push_tx(tx)
        .unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

/// Deserizalizes the bytes contained in a `tx_sync::Request` to a
/// signed sequencer transaction and verifies that the contained
/// sequence action is for the given `expected_chain_id`.
async fn mount_broadcast_tx_sync_mock(
    server: &MockServer,
    expected_chain_id: &'static str,
    expected_nonce: u32,
    expected_tx: Option<Transaction>
) -> MockGuard {
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        native::sequencer::v1alpha1::SignedTransaction,
        Message as _,
    };
    let matcher = move |request: &Request| {
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
        if let Some(tx_data) = &expected_tx {
            assert_eq!(serde_json::from_slice::<Transaction>(sequence_action.data.as_slice()).unwrap(), *tx_data)
        }
        sequence_action.chain_id == expected_chain_id.as_bytes()
            && signed_tx.unsigned_transaction().nonce == expected_nonce
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
