use std::time::Duration;

use astria_core::generated::execution::v1alpha2::{
    Block,
    CommitmentState,
    GenesisInfo,
    GetCommitmentStateRequest,
    GetGenesisInfoRequest,
};
use astria_grpc_mock::{
    matcher::message_type,
    response::constant_response,
    Mock,
};
use bytes::Bytes;
use chrono::Utc;

pub mod helpers;
use helpers::{
    spawn_conductor,
    TestConductor,
};
use sequencer_client::tendermint_rpc::response::Wrapper;
use serde_json::json;
use tokio::{
    join,
    time::timeout,
};
use wiremock::ResponseTemplate;

fn genesis_info() -> GenesisInfo {
    GenesisInfo {
        rollup_id: Bytes::from_static(&[42; 32]),
        sequencer_genesis_block_height: 1,
        celestia_base_block_height: 1,
        celestia_block_variance: 10,
    }
}

fn commitment_state() -> CommitmentState {
    CommitmentState {
        soft: Some(Block {
            number: 1,
            hash: Bytes::from_static(&[69; 128]),
            parent_block_hash: Bytes::from_static(&[69; 128]),
            timestamp: Some(Utc::now().into()),
        }),
        firm: Some(Block {
            number: 1,
            hash: Bytes::from_static(&[69; 128]),
            parent_block_hash: Bytes::from_static(&[69; 128]),
            timestamp: Some(Utc::now().into()),
        }),
    }
}

fn latest_commit() -> Wrapper<sequencer_client::tendermint_rpc::endpoint::commit::Response> {
    use sequencer_client::tendermint::{
        account,
        block::{
            header::{
                Header,
                Version,
            },
            signed_header::SignedHeader,
            Commit,
        },
        chain,
        hash::{
            AppHash,
            Hash,
        },
        time::Time,
    };
    let response = sequencer_client::tendermint_rpc::endpoint::commit::Response {
        signed_header: SignedHeader::new(
            Header {
                version: Version {
                    block: 1,
                    app: 1,
                },
                chain_id: "mocksequencer-1000".parse::<chain::Id>().unwrap(),
                height: 1u32.into(),
                time: Time::now(),
                last_block_id: None,
                last_commit_hash: None,
                data_hash: None,
                validators_hash: Hash::Sha256([0u8; 32]),
                next_validators_hash: Hash::Sha256([0u8; 32]),
                consensus_hash: Hash::Sha256([0u8; 32]),
                app_hash: AppHash::try_from(vec![0u8; 32]).unwrap(),
                last_results_hash: None,
                evidence_hash: None,
                proposer_address: account::Id::new([0u8; 20]),
            },
            Commit {
                height: 1u32.into(),
                ..Commit::default()
            },
        )
        .unwrap(),
        canonical: true,
    };
    Wrapper::new_with_id(
        sequencer_client::tendermint_rpc::Id::uuid_v4(),
        Some(response),
        None,
    )
}

/// Mounts genesis info, commitment state, latest commit mocks and waits for
/// conductor to pick them all up.
async fn mount_and_await_initial_requests(test_conductor: &TestConductor) {
    let genesis_info =
        Mock::for_rpc_given("get_genesis_info", message_type::<GetGenesisInfoRequest>())
            .respond_with(constant_response(genesis_info()))
            .expect(1..)
            .mount_as_scoped(&test_conductor.mock_grpc.mock_server)
            .await;

    let commitment_state = Mock::for_rpc_given(
        "get_commitment_state",
        message_type::<GetCommitmentStateRequest>(),
    )
    .respond_with(constant_response(commitment_state()))
    .expect(1..)
    .mount_as_scoped(&test_conductor.mock_grpc.mock_server)
    .await;

    let latest_commit = wiremock::Mock::given(wiremock::matchers::body_partial_json(
        json!({"method": "commit", "params": {"height": null}}),
    ))
    .respond_with(ResponseTemplate::new(200).set_body_json(latest_commit()))
    .expect(1..)
    .mount_as_scoped(&test_conductor.mock_http)
    .await;

    join!(
        genesis_info.wait_until_satisfied(),
        commitment_state.wait_until_satisfied(),
        latest_commit.wait_until_satisfied(),
    );
}

#[tokio::test]
async fn init_requests() {
    let test_conductor = spawn_conductor().await;

    timeout(
        Duration::from_millis(1000),
        mount_and_await_initial_requests(&test_conductor),
    )
    .await
    .expect("conductor performed all lookups shortly after start");
}
