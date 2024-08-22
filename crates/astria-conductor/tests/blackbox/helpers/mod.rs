use std::time::Duration;

use astria_conductor::{
    conductor,
    config::CommitLevel,
    Conductor,
    Config,
};
use astria_core::{
    brotli::compress_bytes,
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
use celestia_types::{
    nmt::Namespace,
    Blob,
};
use once_cell::sync::Lazy;
use prost::Message;
use sequencer_client::{
    tendermint,
    tendermint_proto,
    tendermint_rpc,
};

#[macro_use]
mod macros;
mod mock_grpc;
use astria_eyre;
pub use mock_grpc::MockGrpc;
use serde_json::json;
use tracing::debug;

pub const CELESTIA_BEARER_TOKEN: &str = "ABCDEFGH";

pub const ROLLUP_ID: RollupId = RollupId::new([42; 32]);
pub static ROLLUP_ID_BYTES: Bytes = Bytes::from_static(&RollupId::get(ROLLUP_ID));

pub const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";

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
    assert_ne!(
        tokio::runtime::Handle::current().runtime_flavor(),
        tokio::runtime::RuntimeFlavor::CurrentThread,
        "conductor must be run on a multi-threaded runtime so that the destructor of the test \
         environment does not stall the runtime: the test could be configured using \
         `#[tokio::test(flavor = \"multi_thread\", worker_threads = 1)]`"
    );
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
        conductor.spawn()
    };

    TestConductor {
        conductor,
        mock_grpc,
        mock_http,
    }
}

pub struct TestConductor {
    pub conductor: conductor::Handle,
    pub mock_grpc: MockGrpc,
    pub mock_http: wiremock::MockServer,
}

impl Drop for TestConductor {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            let err_msg =
                match tokio::time::timeout(Duration::from_secs(2), self.conductor.shutdown()).await
                {
                    Ok(Ok(())) => None,
                    Ok(Err(conductor_err)) => Some(format!(
                        "conductor shut down with an error:\n{conductor_err:?}"
                    )),
                    Err(_timeout) => Some("timed out waiting for conductor to shut down".into()),
                };
            if let Some(err_msg) = err_msg {
                if std::thread::panicking() {
                    debug!("{err_msg}");
                } else {
                    panic!("{err_msg}");
                }
            }
        });
    }
}

impl TestConductor {
    pub async fn mount_abci_info(&self, latest_block_height: u32) {
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
                    response: tendermint::abci::response::Info {
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

    pub async fn mount_get_block<S: serde::Serialize>(
        &self,
        expected_pbjson: S,
        block: astria_core::generated::execution::v1alpha2::Block,
    ) {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        Mock::for_rpc_given("get_block", message_partial_pbjson(&expected_pbjson))
            .respond_with(constant_response(block))
            .expect(1..)
            .mount(&self.mock_grpc.mock_server)
            .await;
    }

    pub async fn mount_celestia_blob_get_all(
        &self,
        celestia_height: u64,
        namespace: Namespace,
        blobs: Vec<Blob>,
    ) {
        use base64::prelude::*;
        use wiremock::{
            matchers::{
                body_partial_json,
                header,
            },
            Mock,
            Request,
            ResponseTemplate,
        };
        let namespace_params = BASE64_STANDARD.encode(namespace.as_bytes());
        Mock::given(body_partial_json(json!({
            "jsonrpc": "2.0",
            "method": "blob.GetAll",
            "params": [celestia_height, [namespace_params]],
        })))
        .and(header(
            "authorization",
            &*format!("Bearer {CELESTIA_BEARER_TOKEN}"),
        ))
        .respond_with(move |request: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let id = body.get("id");
            ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": blobs,
            }))
        })
        .expect(1)
        .mount(&self.mock_http)
        .await;
    }

    pub async fn mount_celestia_header_network_head(
        &self,
        extended_header: celestia_types::ExtendedHeader,
    ) {
        use wiremock::{
            matchers::{
                body_partial_json,
                header,
            },
            Mock,
            ResponseTemplate,
        };
        Mock::given(body_partial_json(
            json!({"jsonrpc": "2.0", "method": "header.NetworkHead"}),
        ))
        .and(header(
            "authorization",
            &*format!("Bearer {CELESTIA_BEARER_TOKEN}"),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": extended_header
        })))
        .expect(1..)
        .mount(&self.mock_http)
        .await;
    }

    pub async fn mount_commit(
        &self,
        signed_header: tendermint::block::signed_header::SignedHeader,
    ) {
        use wiremock::{
            matchers::body_partial_json,
            Mock,
            ResponseTemplate,
        };
        Mock::given(body_partial_json(json!({
            "jsonrpc": "2.0",
            "method": "commit",
            "params": {
                "height": signed_header.header.height.to_string(),
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            tendermint_rpc::response::Wrapper::new_with_id(
                tendermint_rpc::Id::uuid_v4(),
                Some(tendermint_rpc::endpoint::commit::Response {
                    signed_header,
                    canonical: true,
                }),
                None,
            ),
        ))
        .mount(&self.mock_http)
        .await;
    }

    pub async fn mount_genesis(&self) {
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
                            chain_id: SEQUENCER_CHAIN_ID.try_into().unwrap(),
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
        mock_name: Option<&str>,
        expected_pbjson: S,
        response: Block,
    ) -> astria_grpc_mock::MockGuard {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        let mut mock =
            Mock::for_rpc_given("execute_block", message_partial_pbjson(&expected_pbjson))
                .respond_with(constant_response(response));
        if let Some(name) = mock_name {
            mock = mock.with_name(name);
        }
        mock.expect(1)
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
        mock_name: Option<&str>,
        commitment_state: CommitmentState,
    ) -> astria_grpc_mock::MockGuard {
        use astria_core::generated::execution::v1alpha2::UpdateCommitmentStateRequest;
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };
        let mut mock = Mock::for_rpc_given(
            "update_commitment_state",
            message_partial_pbjson(&UpdateCommitmentStateRequest {
                commitment_state: Some(commitment_state.clone()),
            }),
        )
        .respond_with(constant_response(commitment_state.clone()));
        if let Some(name) = mock_name {
            mock = mock.with_name(name);
        }
        mock.expect(1)
            .mount_as_scoped(&self.mock_grpc.mock_server)
            .await
    }

    pub async fn mount_validator_set(
        &self,
        validator_set: tendermint_rpc::endpoint::validators::Response,
    ) {
        use wiremock::{
            matchers::body_partial_json,
            Mock,
            ResponseTemplate,
        };
        Mock::given(body_partial_json(json!({
            "jsonrpc": "2.0",
            "method": "validators",
            "params": {
                "height": validator_set.block_height.to_string(),
                "page": null,
                "per_page": null
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            tendermint_rpc::response::Wrapper::new_with_id(
                tendermint_rpc::Id::uuid_v4(),
                Some(validator_set),
                None,
            ),
        ))
        .mount(&self.mock_http)
        .await;
    }
}

fn make_config() -> Config {
    Config {
        celestia_block_time_ms: 12000,
        celestia_node_http_url: "http://127.0.0.1:26658".into(),
        no_celestia_auth: false,
        celestia_bearer_token: CELESTIA_BEARER_TOKEN.into(),
        sequencer_grpc_url: "http://127.0.0.1:8080".into(),
        sequencer_cometbft_url: "http://127.0.0.1:26657".into(),
        sequencer_requests_per_second: 500,
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

#[must_use]
pub fn make_sequencer_block(height: u32) -> astria_core::sequencerblock::v1alpha1::SequencerBlock {
    fn repeat_bytes_of_u32_as_array(val: u32) -> [u8; 32] {
        let repr = val.to_le_bytes();
        [
            repr[0], repr[1], repr[2], repr[3], repr[0], repr[1], repr[2], repr[3], repr[0],
            repr[1], repr[2], repr[3], repr[0], repr[1], repr[2], repr[3], repr[0], repr[1],
            repr[2], repr[3], repr[0], repr[1], repr[2], repr[3], repr[0], repr[1], repr[2],
            repr[3], repr[0], repr[1], repr[2], repr[3],
        ]
    }

    astria_core::protocol::test_utils::ConfigureSequencerBlock {
        block_hash: Some(repeat_bytes_of_u32_as_array(height)),
        chain_id: Some(crate::SEQUENCER_CHAIN_ID.to_string()),
        height,
        sequence_data: vec![(crate::ROLLUP_ID, data())],
        unix_timestamp: (1i64, 1u32).into(),
        signing_key: Some(signing_key()),
        proposer_address: None,
        ..Default::default()
    }
    .make()
}

pub struct Blobs {
    pub header: Blob,
    pub rollup: Blob,
}

#[must_use]
pub fn make_blobs(heights: &[u32]) -> Blobs {
    use astria_core::generated::sequencerblock::v1alpha1::{
        SubmittedMetadataList,
        SubmittedRollupDataList,
    };
    let mut metadata = Vec::new();
    let mut rollup_data = Vec::new();
    for &height in heights {
        let (head, mut tail) = make_sequencer_block(height).split_for_celestia();
        metadata.push(head.into_raw());
        assert_eq!(
            1,
            tail.len(),
            "this test logic assumes that there is only one rollup in the mocked block"
        );
        rollup_data.push(tail.swap_remove(0).into_raw());
    }
    let header_list = SubmittedMetadataList {
        entries: metadata,
    };
    let rollup_data_list = SubmittedRollupDataList {
        entries: rollup_data,
    };

    let raw_header_list = ::prost::Message::encode_to_vec(&header_list);
    let head_list_compressed = compress_bytes(&raw_header_list).unwrap();
    let header = Blob::new(sequencer_namespace(), head_list_compressed).unwrap();

    let raw_rollup_data_list = ::prost::Message::encode_to_vec(&rollup_data_list);
    let rollup_data_list_compressed = compress_bytes(&raw_rollup_data_list).unwrap();
    let rollup = Blob::new(rollup_namespace(), rollup_data_list_compressed).unwrap();

    Blobs {
        header,
        rollup,
    }
}

fn signing_key() -> astria_core::crypto::SigningKey {
    use rand_chacha::{
        rand_core::SeedableRng as _,
        ChaChaRng,
    };
    astria_core::crypto::SigningKey::new(ChaChaRng::seed_from_u64(0))
}

fn validator() -> tendermint::validator::Info {
    let signing_key = signing_key();
    let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(
        signing_key.verification_key().as_ref(),
    )
    .unwrap();
    let address = tendermint::account::Id::from(pub_key);

    tendermint::validator::Info {
        address,
        pub_key,
        power: 10u32.into(),
        proposer_priority: 0.into(),
        name: None,
    }
}

#[must_use]
pub fn make_commit(height: u32) -> tendermint::block::Commit {
    let signing_key = signing_key();
    let validator = validator();

    let block_hash = make_sequencer_block(height).block_hash();

    let timestamp = tendermint::Time::from_unix_timestamp(1, 1).unwrap();
    let canonical_vote = tendermint::vote::CanonicalVote {
        vote_type: tendermint::vote::Type::Precommit,
        height: height.into(),
        round: 0u16.into(),
        block_id: Some(tendermint::block::Id {
            hash: tendermint::Hash::Sha256(block_hash),
            part_set_header: tendermint::block::parts::Header::default(),
        }),
        timestamp: Some(timestamp),
        chain_id: crate::SEQUENCER_CHAIN_ID.try_into().unwrap(),
    };

    let message = tendermint_proto::types::CanonicalVote::from(canonical_vote)
        .encode_length_delimited_to_vec();
    let signature = signing_key.sign(&message);

    tendermint::block::Commit {
        height: height.into(),
        round: 0u16.into(),
        block_id: tendermint::block::Id {
            hash: tendermint::Hash::Sha256(block_hash),
            part_set_header: tendermint::block::parts::Header::default(),
        },
        signatures: vec![tendermint::block::CommitSig::BlockIdFlagCommit {
            validator_address: validator.address,
            timestamp,
            signature: Some(signature.to_bytes().as_ref().try_into().unwrap()),
        }],
    }
}

#[must_use]
pub fn make_signed_header(height: u32) -> tendermint::block::signed_header::SignedHeader {
    tendermint::block::signed_header::SignedHeader::new(
        tendermint::block::Header {
            version: tendermint::block::header::Version {
                block: 1,
                app: 1,
            },
            chain_id: crate::SEQUENCER_CHAIN_ID.try_into().unwrap(),
            height: height.into(),
            time: tendermint::time::Time::from_unix_timestamp(1, 1).unwrap(),
            last_block_id: None,
            last_commit_hash: None,
            data_hash: None,
            validators_hash: tendermint::Hash::Sha256([0; 32]),
            next_validators_hash: tendermint::Hash::Sha256([0; 32]),
            consensus_hash: tendermint::Hash::Sha256([0; 32]),
            app_hash: tendermint::AppHash::default(),
            last_results_hash: None,
            evidence_hash: None,
            proposer_address: validator().address,
        },
        make_commit(height),
    )
    .unwrap()
}

#[must_use]
pub fn data() -> Vec<u8> {
    b"hello_world".to_vec()
}

#[must_use]
pub fn make_validator_set(height: u32) -> tendermint_rpc::endpoint::validators::Response {
    tendermint_rpc::endpoint::validators::Response::new(height.into(), vec![validator()], 1)
}

#[must_use]
pub fn rollup_namespace() -> Namespace {
    astria_core::celestia::namespace_v0_from_rollup_id(ROLLUP_ID)
}

#[must_use]
pub fn sequencer_namespace() -> Namespace {
    astria_core::celestia::namespace_v0_from_sha256_of_bytes(SEQUENCER_CHAIN_ID.as_bytes())
}
