use std::time::Duration;

use astria_sequencer::transaction;
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
async fn pending_eth_tx_is_submitted_to_sequencer() {
    let test_composer = spawn_composer().await;
    let mock_guard = mount_broadcast_tx_sync_mock(&test_composer.sequencer).await;
    test_composer.geth.push_tx(Transaction::default()).unwrap();
    tokio::time::timeout(
        Duration::from_millis(100),
        mock_guard.wait_until_satisfied(),
    )
    .await
    .expect("mocked sequencer should have received a broadcast message from composer");
}

async fn mount_broadcast_tx_sync_mock(server: &MockServer) -> MockGuard {
    let matcher = |request: &Request| {
        let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
            serde_json::from_slice(&request.body)
                .expect("can't deserialize to JSONRPC wrapped tx_sync::Request");
        let signed_tx = transaction::Signed::try_from_slice(&wrapped_tx_sync_req.params().tx)
            .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
        debug!(?signed_tx, "sequencer mock received signed transaction");
        true
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
