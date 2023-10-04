use proto::Message;
use serde_json::json;
use tendermint_rpc::{
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
    use proto::generated::sequencer::v1alpha1::NonceResponse;
    let server = MockServer::start().await;
    mount_abci_query_mock(
        &server,
        "accounts/nonce",
        NonceResponse {
            height: 42,
            nonce: 42,
        },
    )
    .await;
    server
}

pub async fn mount_abci_query_mock(server: &MockServer, query_path: &str, response: impl Message) {
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
        .expect(1)
        .mount(server)
        .await;
}
