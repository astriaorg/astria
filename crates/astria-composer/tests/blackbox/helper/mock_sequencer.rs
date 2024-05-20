use prost::Message;
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
    MockGuard,
    MockServer,
    ResponseTemplate,
};

pub async fn start() -> (MockServer, MockGuard) {
    use astria_core::generated::protocol::account::v1alpha1::NonceResponse;
    let server = MockServer::start().await;
    let startup_guard = mount_abci_query_mock(
        &server,
        "accounts/nonce",
        NonceResponse {
            height: 0,
            nonce: 0,
        },
    )
    .await;
    (server, startup_guard)
}

pub async fn mount_abci_query_mock(
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
