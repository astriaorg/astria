use astria_conductor::{
    config::CommitLevel,
    Conductor,
    Config,
};
use astria_core::{
    generated::{
        execution::v1alpha2::{
            Block,
            CommitmentState,
            GenesisInfo,
        },
        sequencerblock::v1alpha1::FilteredSequencerBlock,
    },
    primitive::v1::RollupId,
};
use bytes::Bytes;
use once_cell::sync::Lazy;

#[macro_use]
mod macros;
mod mock_grpc;
pub use mock_grpc::MockGrpc;
use serde_json::json;
use tokio::task::JoinHandle;

pub const CELESTIA_BEARER_TOKEN: &str = "ABCDEFGH";

pub const ROLLUP_ID: RollupId = RollupId::new([42; 32]);
pub static ROLLUP_ID_BYTES: Bytes = Bytes::from_static(&RollupId::get(ROLLUP_ID));

pub const INITIAL_SOFT_HASH: [u8; 64] = [1; 64];
pub const INITIAL_FIRM_HASH: [u8; 64] = [1; 64];

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    astria_eyre::install().unwrap();
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        println!("initializing telemetry");
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .force_stdout()
            .pretty_print()
            .filter_directives(&filter_directives)
            .try_init()
            .unwrap();
    } else {
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::sink)
            .try_init()
            .unwrap();
    }
});

pub async fn spawn_conductor(execution_commit_level: CommitLevel) -> TestConductor {
    Lazy::force(&TELEMETRY);

    let mock_grpc = MockGrpc::spawn().await;
    let mock_http = wiremock::MockServer::start().await;

    let config = Config {
        celestia_node_http_url: mock_http.uri(),
        execution_rpc_url: format!("http://{}", mock_grpc.local_addr),
        sequencer_cometbft_url: mock_http.uri(),
        sequencer_grpc_url: format!("http://{}", mock_grpc.local_addr),
        execution_commit_level,
        ..make_config()
    };

    let conductor = {
        let conductor = Conductor::new(config).unwrap();
        tokio::spawn(conductor.run_until_stopped())
    };

    TestConductor {
        conductor,
        mock_grpc,
        mock_http,
    }
}

pub struct TestConductor {
    pub conductor: JoinHandle<()>,
    pub mock_grpc: MockGrpc,
    pub mock_http: wiremock::MockServer,
}

impl TestConductor {
    pub async fn mount_abci_info(&self, latest_block_height: u32) {
        use sequencer_client::{
            tendermint::abci,
            tendermint_rpc::{
                self,
            },
        };
        use wiremock::{
            matchers::body_partial_json,
            Mock,
            ResponseTemplate,
        };
        Mock::given(body_partial_json(
            json!({"jsonrpc": "2.0", "method": "abci_info", "params": null}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            tendermint_rpc::response::Wrapper::new_with_id(
                tendermint_rpc::Id::uuid_v4(),
                Some(tendermint_rpc::endpoint::abci_info::Response {
                    response: abci::response::Info {
                        last_block_height: latest_block_height.into(),
                        ..Default::default()
                    },
                }),
                None,
            ),
        ))
        .expect(1..)
        .mount(&self.mock_http)
        .await;
    }

    pub async fn mount_get_genesis_info(&self, genesis_info: GenesisInfo) {
        use astria_core::generated::execution::v1alpha2::GetGenesisInfoRequest;
        astria_grpc_mock::Mock::for_rpc_given(
            "get_genesis_info",
            astria_grpc_mock::matcher::message_type::<GetGenesisInfoRequest>(),
        )
        .respond_with(astria_grpc_mock::response::constant_response(genesis_info))
        .expect(1..)
        .mount(&self.mock_grpc.mock_server)
        .await;
    }

    pub async fn mount_get_commitment_state(&self, commitment_state: CommitmentState) {
        use astria_core::generated::execution::v1alpha2::GetCommitmentStateRequest;

        astria_grpc_mock::Mock::for_rpc_given(
            "get_commitment_state",
            astria_grpc_mock::matcher::message_type::<GetCommitmentStateRequest>(),
        )
        .respond_with(astria_grpc_mock::response::constant_response(
            commitment_state,
        ))
        .expect(1..)
        .mount(&self.mock_grpc.mock_server)
        .await;
    }

    pub async fn mount_execute_block<S: serde::Serialize>(
        &self,
        expected_pbjson: S,
        response: Block,
    ) -> astria_grpc_mock::MockGuard {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        Mock::for_rpc_given("execute_block", message_partial_pbjson(&expected_pbjson))
            .respond_with(constant_response(response))
            .expect(1)
            .mount_as_scoped(&self.mock_grpc.mock_server)
            .await
    }

    pub async fn mount_get_filtered_sequencer_block<S: serde::Serialize>(
        &self,
        expected_pbjson: S,
        response: FilteredSequencerBlock,
    ) {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        Mock::for_rpc_given(
            "get_filtered_sequencer_block",
            message_partial_pbjson(&expected_pbjson),
        )
        .respond_with(constant_response(response))
        .expect(1..)
        .mount(&self.mock_grpc.mock_server)
        .await;
    }

    pub async fn mount_update_commitment_state(
        &self,
        commitment_state: CommitmentState,
    ) -> astria_grpc_mock::MockGuard {
        use astria_core::generated::execution::v1alpha2::UpdateCommitmentStateRequest;
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        Mock::for_rpc_given(
            "update_commitment_state",
            message_partial_pbjson(&UpdateCommitmentStateRequest {
                commitment_state: Some(commitment_state.clone()),
            }),
        )
        .respond_with(constant_response(commitment_state.clone()))
        .expect(1)
        .mount_as_scoped(&self.mock_grpc.mock_server)
        .await
    }
}

fn make_config() -> Config {
    Config {
        celestia_block_time_ms: 12000,
        celestia_node_http_url: "http://127.0.0.1:26658".into(),
        celestia_bearer_token: CELESTIA_BEARER_TOKEN.into(),
        sequencer_grpc_url: "http://127.0.0.1:8080".into(),
        sequencer_cometbft_url: "http://127.0.0.1:26657".into(),
        sequencer_block_time_ms: 2000,
        execution_rpc_url: "http://127.0.0.1:50051".into(),
        log: "info".into(),
        execution_commit_level: astria_conductor::config::CommitLevel::SoftAndFirm,
        force_stdout: false,
        no_otel: false,
        no_metrics: true,
        metrics_http_listener_addr: String::new(),
        pretty_print: false,
    }
}
