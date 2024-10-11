use std::time::Duration;

use serde_json::json;
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
use wiremock::{
    matchers::body_partial_json,
    Mock,
    MockServer,
    ResponseTemplate,
};

pub async fn start() -> MockServer {
    let server = MockServer::start().await;
    mount_genesis(&server, "test-chain-1").await;
    server
}

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
