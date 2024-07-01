use core::time::Duration;
use std::sync::OnceLock;

use serde_json::json;
use tendermint_rpc::HttpClient;
use wiremock::MockServer;

use crate::{
    config::CommitLevel,
    executor,
    metrics::Metrics,
    sequencer,
    sequencer::CancellationToken,
};

async fn mount_genesis(mock_http: &MockServer, sequencer_chain_id: &str) {
    use tendermint::{
        consensus::{
            params::{
                AbciParams,
                ValidatorParams,
            },
            Params,
        },
        genesis::Genesis,
        time::Time,
    };
    use wiremock::{
        matchers::body_partial_json,
        Mock,
        ResponseTemplate,
    };
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
                        chain_id: sequencer_chain_id.try_into().unwrap(),
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
    .mount(mock_http)
    .await;
}

#[tokio::test]
async fn should_exit_on_chain_id_mismatch() {
    static METRICS: OnceLock<Metrics> = OnceLock::new();
    let metrics = METRICS.get_or_init(Metrics::new);
    let mock_http = wiremock::MockServer::start().await;
    let sequencer_grpc_client =
        sequencer::SequencerGrpcClient::new("http://127.0.0.1:8080").unwrap();
    let sequencer_cometbft_client = HttpClient::new(&*mock_http.uri()).unwrap();
    let shutdown = CancellationToken::new();

    mount_genesis(&mock_http, "bad-id").await;

    let (_executor, handle) = executor::Builder {
        mode: CommitLevel::SoftAndFirm,
        rollup_address: "http://127.0.0.1:50051".to_string(),
        shutdown: shutdown.clone(),
        metrics,
    }
    .build()
    .unwrap();

    let sequencer_reader = sequencer::Builder {
        sequencer_grpc_client,
        sequencer_cometbft_client: sequencer_cometbft_client.clone(),
        sequencer_block_time: Duration::from_millis(2000),
        sequencer_chain_id: "test_sequencer-1000".to_string(),
        shutdown: shutdown.clone(),
        executor: handle,
    }
    .build();

    assert!(sequencer_reader.run_until_stopped().await.is_err());
}
