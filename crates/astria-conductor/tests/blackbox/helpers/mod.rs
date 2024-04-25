use std::time::Duration;

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
    brotli::compress_bytes,
};
use bytes::Bytes;
use celestia_client::celestia_types::{
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
use tokio::task::JoinHandle;

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
        extended_header: celestia_client::celestia_types::ExtendedHeader,
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

#[must_use]
pub fn make_sequencer_block(height: u32) -> astria_core::sequencerblock::v1alpha1::SequencerBlock {
    astria_core::sequencerblock::v1alpha1::SequencerBlock::try_from_cometbft(
        astria_core::protocol::test_utils::ConfigureCometBftBlock {
            chain_id: Some(crate::SEQUENCER_CHAIN_ID.to_string()),
            height,
            rollup_transactions: vec![(crate::ROLLUP_ID, data())],
            unix_timestamp: (1i64, 1u32).into(),
            signing_key: Some(signing_key()),
            proposer_address: None,
        }
        .make(),
    )
    .unwrap()
}

pub struct Blobs {
    pub header: Vec<Blob>,
    pub rollup: Vec<Blob>,
}

#[must_use]
pub fn make_blobs(height: u32) -> Blobs {
    let (head, tail) = make_sequencer_block(height).into_celestia_blobs();

    let raw_header = ::prost::Message::encode_to_vec(&head.into_raw());
    let head_compressed = compress_bytes(&raw_header).unwrap();
    let header = ::celestia_client::celestia_types::Blob::new(
        ::celestia_client::celestia_namespace_v0_from_bytes(crate::SEQUENCER_CHAIN_ID.as_bytes()),
        head_compressed,
    )
    .unwrap();

    let mut rollup = Vec::new();
    for elem in tail {
        let raw_rollup = ::prost::Message::encode_to_vec(&elem.into_raw());
        let rollup_compressed = compress_bytes(&raw_rollup).unwrap();
        let blob = ::celestia_client::celestia_types::Blob::new(
            ::celestia_client::celestia_namespace_v0_from_rollup_id(crate::ROLLUP_ID),
            rollup_compressed,
        )
        .unwrap();
        rollup.push(blob);
    }
    Blobs {
        header: vec![header],
        rollup,
    }
}

fn signing_key() -> ed25519_consensus::SigningKey {
    use rand_chacha::{
        rand_core::SeedableRng as _,
        ChaChaRng,
    };
    ed25519_consensus::SigningKey::new(ChaChaRng::seed_from_u64(0))
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
            signature: Some(signature.into()),
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
    celestia_client::celestia_namespace_v0_from_rollup_id(ROLLUP_ID)
}

#[must_use]
pub fn sequencer_namespace() -> Namespace {
    celestia_client::celestia_namespace_v0_from_bytes(SEQUENCER_CHAIN_ID.as_bytes())
}
