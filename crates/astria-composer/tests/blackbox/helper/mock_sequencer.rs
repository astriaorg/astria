use astria_sequencer::accounts::{
    query,
    types::Nonce,
};
use serde_json::json;
use tendermint::{
    abci,
    AppHash,
};
use tendermint_rpc::{
    endpoint::abci_info,
    response,
    Id,
};
use wiremock::{
    matchers::{
        body_partial_json,
        body_string_contains,
    },
    Mock,
    MockServer,
    ResponseTemplate,
};

pub async fn start() -> MockServer {
    let server = MockServer::start().await;
    mount_abci_info_mock(&server).await;
    mount_abci_query_mock(
        &server,
        "accounts/nonce",
        &query::Response::NonceResponse(Nonce::from(0)),
    )
    .await;
    server
}

async fn mount_abci_info_mock(server: &MockServer) {
    let abci_response = abci_info::Response {
        response: abci::response::Info {
            data: "SequencerRelayerTest".into(),
            version: "1.0.0".into(),
            app_version: 1,
            last_block_height: 5u32.into(),
            last_block_app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        },
    };
    let abci_response = response::Wrapper::new_with_id(Id::Num(1), Some(abci_response), None);
    Mock::given(body_partial_json(json!({"method": "abci_info"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(abci_response))
        .expect(1..)
        .mount(server)
        .await;
}

async fn mount_abci_query_mock(server: &MockServer, query_path: &str, response: &query::Response) {
    use borsh::BorshSerialize as _;
    let expected_body = json!({
        "method": "abci_query"
    });
    let response = tendermint_rpc::endpoint::abci_query::Response {
        response: tendermint_rpc::endpoint::abci_query::AbciQuery {
            value: response.try_to_vec().unwrap(),
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
        .mount(server)
        .await;
}
