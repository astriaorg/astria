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
use tendermint_rpc::endpoint::status;

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
    mount_cometbft_status_response(&server, "test-chain-1").await;
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

async fn mount_cometbft_status_response(
    server: &MockServer,
    mock_sequencer_chain_id: &str,
) {
    let mut status_response: status::Response = serde_json::from_value(json!({
        "node_info": {
            "protocol_version": {
                "p2p": "8",
                "block": "11",
                "app": "0"
            },
            "id": "a1d3bbddb7800c6da2e64169fec281494e963ba3",
            "listen_addr": "tcp://0.0.0.0:26656",
            "network": "test",
            "version": "0.38.6",
            "channels": "40202122233038606100",
            "moniker": "fullnode",
            "other": {
                "tx_index": "on",
                "rpc_address": "tcp://0.0.0.0:26657"
            }
        },
        "sync_info": {
            "latest_block_hash": "A4202E4E367712AC2A797860265A7EBEA8A3ACE513CB0105C2C9058449641202",
            "latest_app_hash": "BCC9C9B82A49EC37AADA41D32B4FBECD2441563703955413195BDA2236775A68",
            "latest_block_height": "452605",
            "latest_block_time": "2024-05-09T15:59:17.849713071Z",
            "earliest_block_hash": "C34B7B0B82423554B844F444044D7D08A026D6E413E6F72848DB2F8C77ACE165",
            "earliest_app_hash": "6B776065775471CEF46AC75DE09A4B869A0E0EB1D7725A04A342C0E46C16F472",
            "earliest_block_height": "1",
            "earliest_block_time": "2024-04-23T00:49:11.964127Z",
            "catching_up": false
        },
        "validator_info": {
            "address": "0B46F33BA2FA5C2E2AD4C4C4E5ECE3F1CA03D195",
            "pub_key": {
                "type": "tendermint/PubKeyEd25519",
                "value": "bA6GipHUijVuiYhv+4XymdePBsn8EeTqjGqNQrBGZ4I="
            },
            "voting_power": "0"
        }
    })).unwrap();
    status_response.node_info.network = mock_sequencer_chain_id.to_string().parse().unwrap();

    let response = tendermint_rpc::response::Wrapper::new_with_id(
        Id::Num(1),
        Some(status_response),
        None,
    );

    Mock::given(body_partial_json(json!({"method": "status"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .up_to_n_times(1)
        .expect(1..)
        .named("CometBFT status")
        .mount(server)
        .await
}
