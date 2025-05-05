use std::{
    cmp::max,
    sync::LazyLock,
    time::Duration,
};

use astria_conductor::{
    conductor,
    config::CommitLevel,
    Conductor,
    Config,
    Metrics,
};
use astria_core::{
    brotli::compress_bytes,
    generated::astria::{
        execution::v2::{
            CommitmentState,
            ExecutedBlockMetadata,
            ExecutionSession,
        },
        sequencerblock::v1::FilteredSequencerBlock,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::block,
};
use astria_grpc_mock::response::error_response;
use base64::{
    prelude::BASE64_STANDARD,
    Engine as _,
};
use bytes::Bytes;
use celestia_types::{
    nmt::Namespace,
    Blob,
};
use prost::Message;
use sequencer_client::{
    tendermint,
    tendermint_proto,
    tendermint_rpc,
};
use telemetry::metrics;

#[macro_use]
mod macros;
mod mock_grpc;
use astria_eyre;
pub use mock_grpc::MockGrpc;
use serde_json::json;
use tracing::debug;
use wiremock::MockServer;

pub const CELESTIA_BEARER_TOKEN: &str = "ABCDEFGH";

pub const ROLLUP_ID: RollupId = RollupId::new([42; 32]);
pub static ROLLUP_ID_BYTES: Bytes = Bytes::from_static(ROLLUP_ID.as_bytes());

pub const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";
pub const CELESTIA_CHAIN_ID: &str = "test_celestia-1000";
pub const EXECUTION_SESSION_ID: &str = "test_execution_session";

pub const INITIAL_SOFT_HASH: [u8; 64] = [1; 64];
pub const INITIAL_FIRM_HASH: [u8; 64] = [1; 64];

static TELEMETRY: LazyLock<()> = LazyLock::new(|| {
    astria_eyre::install().unwrap();
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        println!("initializing telemetry");
        let _ = telemetry::configure()
            .set_no_otel(true)
            .set_force_stdout(true)
            .set_filter_directives(&filter_directives)
            .try_init::<Metrics>(&())
            .unwrap();
    } else {
        let _ = telemetry::configure()
            .set_no_otel(true)
            .set_stdout_writer(std::io::sink)
            .try_init::<Metrics>(&())
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
    LazyLock::force(&TELEMETRY);

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

    let (metrics, metrics_handle) = metrics::ConfigBuilder::new()
        .set_global_recorder(false)
        .build(&())
        .unwrap();
    let metrics = Box::leak(Box::new(metrics));

    let conductor = {
        let conductor = Conductor::new(config, metrics).unwrap();
        conductor.spawn()
    };

    let state = TestRollupState {
        soft_initializer: 1,
        firm_initializer: 1,
        firm_number: 0,
    };

    TestConductor {
        conductor,
        mock_grpc,
        mock_http,
        metrics_handle,
        state,
    }
}

struct TestRollupState {
    soft_initializer: u8,
    firm_initializer: u8,
    firm_number: u32,
}

pub struct TestConductor {
    pub conductor: conductor::Handle,
    pub mock_grpc: MockGrpc,
    pub mock_http: wiremock::MockServer,
    pub metrics_handle: metrics::Handle,
    state: TestRollupState,
}

impl Drop for TestConductor {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            let err_msg =
                match tokio::time::timeout(Duration::from_secs(2), self.conductor.shutdown()).await
                {
                    Ok(Ok(_)) => None,
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
    pub fn put_rollup_state_hash_initializers(&mut self, firm: u8, soft: u8) {
        self.state.firm_initializer = firm;
        self.state.soft_initializer = soft;
    }

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

    pub async fn mount_get_executed_block_metadata<S: serde::Serialize>(
        &self,
        number: u32,
    ) {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };

        let expected_pbjson = GetBlockRequest {
            identifier: Some(BlockIdentifier {
                identifier: Some(Identifier::BlockNumber(number)),
            }),
        };

        // Calculate difference between firm block number and firm initializer, applying to given
        // number to obtain its hash initializer. This accomodates potential use cases where the
        // hash initializer does not match the block number.
        let delta = i64::from(self.state.firm_number)
            .saturating_sub(i64::from(self.state.firm_initializer));
        let hash_initializer = u8::try_from(i64::from(number).saturating_sub(delta)).expect(
            "should be able to derive `u8` hash initializer from `number + (firm_number - \
             firm_initializer)`",
        );

        Mock::for_rpc_given("get_block", message_partial_pbjson(&expected_pbjson))
            .respond_with(constant_response(block!(
                number: number,
                hash: [hash_initializer; 64],
                parent: [hash_initializer.saturating_sub(1); 64],
            )))
            .expect(1..)
            .mount(&self.mock_grpc.mock_server)
            .await;
    }

    pub async fn mount_celestia_blob_get_all(
        &self,
        celestia_height: u64,
        namespace: Namespace,
        blobs: Vec<Blob>,
        delay: Option<Duration>,
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
        let delay = delay.unwrap_or(Duration::from_millis(0));
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
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": blobs,
                }))
                .set_delay(delay)
        })
        .expect(1..)
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
            Request,
            ResponseTemplate,
        };
        Mock::given(body_partial_json(
            json!({"jsonrpc": "2.0", "method": "header.NetworkHead"}),
        ))
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
                "result": extended_header
            }))
        })
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

    pub async fn mount_genesis(&self, chain_id: &str) {
        mount_genesis(&self.mock_http, chain_id).await;
    }

    pub async fn mount_create_execution_session(
        &self,
        execution_session: ExecutionSession,
        up_to_n_times: u64,
        expected_calls: u64,
    ) {
        use astria_core::generated::astria::execution::v2::CreateExecutionSessionRequest;
        astria_grpc_mock::Mock::for_rpc_given(
            "create_execution_session",
            astria_grpc_mock::matcher::message_type::<CreateExecutionSessionRequest>(),
        )
        .respond_with(astria_grpc_mock::response::constant_response(
            execution_session,
        ))
        .up_to_n_times(up_to_n_times)
        .expect(expected_calls)
        .mount(&self.mock_grpc.mock_server)
        .await;
    }

    pub async fn mount_execute_block(
        &self,
        mock_name: Option<&str>,
        number: u32,
    ) -> astria_grpc_mock::MockGuard {
        use astria_core::generated::astria::execution::v2::ExecuteBlockResponse;
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };

        let parent_initializer = max(self.state.soft_initializer, self.state.firm_initializer);
        let response = block!(
            number: number,
            hash: [parent_initializer.saturating_add(1); 64],
            parent: [parent_initializer; 64],
        );

        let mut mock = Mock::for_rpc_given(
            "execute_block",
            message_partial_pbjson(&json!({
                "prevBlockHash": BASE64_STANDARD.encode([parent_initializer; 64]),
                "transactions": [{"sequencedData": BASE64_STANDARD.encode(data())}],
            })),
        )
        .respond_with(constant_response(response));
        if let Some(name) = mock_name {
            mock = mock.with_name(name);
        }
        mock.expect(expected_calls)
            .mount_as_scoped(&self.mock_grpc.mock_server)
            .await
    }

    pub async fn mount_get_filtered_sequencer_block<S: serde::Serialize>(
        &self,
        expected_pbjson: S,
        response: FilteredSequencerBlock,
        delay: Duration,
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
        .respond_with(constant_response(response).set_delay(delay))
        .expect(1..)
        .mount(&self.mock_grpc.mock_server)
        .await;
    }

    pub async fn mount_update_commitment_state(
        &mut self,
        mock_name: Option<&str>,
        number: u32,
        is_with_firm: bool,
        base_celestia_height: u64,
        expected_calls: u64,
    ) -> astria_grpc_mock::MockGuard {
        use astria_core::generated::astria::execution::v2::UpdateCommitmentStateRequest;
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            response::constant_response,
            Mock,
        };

        // If this is a firm commitment update, we want to match the soft info to firm. If not,
        // we leave the firm info as is.
        let (firm_number, soft_initializer) = if is_with_firm {
            self.state.firm_number = number;
            self.state.firm_initializer = self.state.firm_initializer.saturating_add(1);
            (number, self.state.firm_initializer) // Set soft initializer to firm initializer
        } else {
            self.state.soft_initializer = self.state.soft_initializer.saturating_add(1);
            (self.state.firm_number, self.state.soft_initializer)
        };

        let commitment_state = commitment_state!(
            firm: (
                number: firm_number,
                hash: [self.state.firm_initializer; 64],
                parent: [self.state.firm_initializer.saturating_sub(1); 64],
            ),
            soft: (
                number: number,
                hash: [soft_initializer; 64],
                parent: [soft_initializer.saturating_sub(1); 64],
            ),
            base_celestia_height: base_celestia_height,
        );

        let mut mock = Mock::for_rpc_given(
            "update_commitment_state",
            message_partial_pbjson(&UpdateCommitmentStateRequest {
                session_id: EXECUTION_SESSION_ID.to_string(),
                commitment_state: Some(commitment_state.clone()),
            }),
        )
        .respond_with(constant_response(commitment_state.clone()));
        if let Some(name) = mock_name {
            mock = mock.with_name(name);
        }
        mock.expect(expected_calls)
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

    pub async fn mount_tonic_status_code<S: serde::Serialize>(
        &self,
        expected_pbjson: S,
        code: tonic::Code,
    ) -> astria_grpc_mock::MockGuard {
        use astria_grpc_mock::{
            matcher::message_partial_pbjson,
            Mock,
        };

        let mock = Mock::for_rpc_given("execute_block", message_partial_pbjson(&expected_pbjson))
            .respond_with(error_response(code))
            .up_to_n_times(1);
        mock.expect(1)
            .mount_as_scoped(&self.mock_grpc.mock_server)
            .await
    }
}

pub async fn mount_genesis(mock_http: &MockServer, chain_id: &str) {
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
                        chain_id: chain_id.try_into().unwrap(),
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

pub(crate) fn make_config() -> Config {
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
    }
}

#[must_use]
pub fn make_sequencer_block(height: u32) -> astria_core::sequencerblock::v1::SequencerBlock {
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
        block_hash: Some(block::Hash::new(repeat_bytes_of_u32_as_array(height))),
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
    use astria_core::generated::astria::sequencerblock::v1::{
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
    let header = Blob::new(
        sequencer_namespace(),
        head_list_compressed,
        celestia_types::AppVersion::V3,
    )
    .unwrap();

    let raw_rollup_data_list = ::prost::Message::encode_to_vec(&rollup_data_list);
    let rollup_data_list_compressed = compress_bytes(&raw_rollup_data_list).unwrap();
    let rollup = Blob::new(
        rollup_namespace(),
        rollup_data_list_compressed,
        celestia_types::AppVersion::V3,
    )
    .unwrap();

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

    let block_hash = *make_sequencer_block(height).block_hash();

    let timestamp = tendermint::Time::from_unix_timestamp(1, 1).unwrap();
    let canonical_vote = tendermint::vote::CanonicalVote {
        vote_type: tendermint::vote::Type::Precommit,
        height: height.into(),
        round: 0u16.into(),
        block_id: Some(tendermint::block::Id {
            hash: tendermint::Hash::Sha256(block_hash.get()),
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
            hash: tendermint::Hash::Sha256(block_hash.get()),
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
