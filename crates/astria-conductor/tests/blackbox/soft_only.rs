use std::time::Duration;

use astria_conductor::{
    config::CommitLevel,
    Conductor,
    Config,
};
use astria_core::generated::astria::execution::v2::{
    GetCommitmentStateRequest,
    GetGenesisInfoRequest,
};
use futures::future::{
    join,
    join4,
};
use serde_json::json;
use telemetry::metrics;
use tokio::time::timeout;

use crate::{
    commitment_state,
    genesis_info,
    helpers::{
        genesis_responder,
        make_config,
        spawn_conductor,
        MockGrpc,
    },
    SEQUENCER_CHAIN_ID,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor.mock_abci_info(3).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;

    let execute_block = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 2,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn submits_two_heights_in_succession() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor.mock_abci_info(4).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;
    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .mount()
        .await;

    let execute_block_number_2 = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .named("first_execute")
        .mount_as_scoped()
        .await;

    let update_commitment_state_number_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 2,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("first_update")
        .mount_as_scoped()
        .await;

    let execute_block_number_3 = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .named("second_execute")
        .mount_as_scoped()
        .await;

    let update_commitment_state_number_3 = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 3,
                hash: [3; 64],
                parent: [2; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("second_update")
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join4(
            execute_block_number_2.wait_until_satisfied(),
            update_commitment_state_number_2.wait_until_satisfied(),
            execute_block_number_3.wait_until_satisfied(),
            update_commitment_state_number_3.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn skips_already_executed_heights() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 9,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 5,
                hash: [1; 64],
                parent: [0; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor.mock_abci_info(7).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(7)
        .mount()
        .await;

    let execute_block = test_conductor
        .mock_execute_block(6, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 6,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn requests_from_later_genesis_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 12,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 10,
        ))
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor.mock_abci_info(12).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(12)
        .mount()
        .await;

    let execute_block = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .mount_as_scoped()
        .await;

    let update_commitment_state = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 2,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the soft block and updated the soft commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn exits_on_sequencer_chain_id_mismatch() {
    use astria_grpc_mock::{
        matcher,
        response as GrpcResponse,
        Mock as GrpcMock,
    };

    // FIXME (https://github.com/astriaorg/astria/issues/1602)
    // We have to create our own test conductor and perform mounts manually because `TestConductor`
    // implements the `Drop` trait, which disallows us from taking ownership of its tasks and
    // awaiting their completion.

    let mock_grpc = MockGrpc::spawn().await;
    let mock_http = wiremock::MockServer::start().await;

    let config = Config {
        celestia_node_http_url: mock_http.uri(),
        execution_rpc_url: format!("http://{}", mock_grpc.local_addr),
        sequencer_cometbft_url: mock_http.uri(),
        sequencer_grpc_url: format!("http://{}", mock_grpc.local_addr),
        execution_commit_level: CommitLevel::SoftOnly,
        ..make_config()
    };

    let (metrics, _) = metrics::ConfigBuilder::new()
        .set_global_recorder(false)
        .build(&())
        .unwrap();
    let metrics = Box::leak(Box::new(metrics));

    let conductor = {
        let conductor = Conductor::new(config, metrics).unwrap();
        conductor.spawn()
    };

    GrpcMock::for_rpc_given(
        "get_genesis_info",
        matcher::message_type::<GetGenesisInfoRequest>(),
    )
    .respond_with(GrpcResponse::constant_response(
        genesis_info!(sequencer_start_height: 3,
            celestia_block_variance: 10,
        rollup_start_block_number: 2,
    rollup_stop_block_number: 9),
    ))
    .expect(0..)
    .mount(&mock_grpc.mock_server)
    .await;

    GrpcMock::for_rpc_given(
        "get_commitment_state",
        matcher::message_type::<GetCommitmentStateRequest>(),
    )
    .respond_with(GrpcResponse::constant_response(commitment_state!(firm: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 1,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,)))
    .expect(0..)
    .mount(&mock_grpc.mock_server)
    .await;

    let bad_chain_id = "bad_chain_id";
    wiremock::Mock::given(wiremock::matchers::body_partial_json(
        json!({"jsonrpc": "2.0", "method": "genesis", "params": null}),
    ))
    .respond_with(genesis_responder(bad_chain_id))
    .expect(1..)
    .mount(&mock_http)
    .await;

    let res = conductor.await;
    match res {
        Ok(()) => panic!("conductor should have exited with an error, no error received"),
        Err(e) => {
            let mut source = e.source();
            while source.is_some() {
                let err = source.unwrap();
                if err.to_string().contains(
                    format!(
                        "expected chain id `{SEQUENCER_CHAIN_ID}` does not match actual: \
                         `{bad_chain_id}`"
                    )
                    .as_str(),
                ) {
                    return;
                }
                source = err.source();
            }
            panic!("expected exit due to chain ID mismatch, but got a different error: {e:?}")
        }
    }
}

/// Tests that the conductor correctly stops at the sequencer stop block height in soft only mode,
/// executing the soft block at that height. Then, tests that the conductor correctly restarts
/// and continues executing soft blocks after receiving updated genesis info and commitment state.
///
/// It consists of the following steps:
/// 1. Mount commitment state and genesis info with a stop height of 3, responding only up to 1 time
///    so that the same information is not retrieved after restarting.
/// 2. Mount sequencer genesis, ABCI info, and sequencer blocks for heights 3 and 4.
/// 3. Mount `execute_block` and `update_commitment_state` mocks for the soft block at height 3,
///    expecting only 1 call and timing out after 1000ms.
/// 4. Mount updated commitment state and genesis info with a stop height of 10 (more than high
///    enough) and a rollup start block height of 2, reflecting that the first block has already
///    been executed.
/// 5. Mount `execute_block` and `update_commitment_state` mocks for the soft block at height 4,
///    awaiting their satisfaction.
#[expect(
    clippy::too_many_lines,
    reason = "All lines reasonably necessary for the thoroughness of this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restarts_after_reaching_stop_block_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 3,
            celestia_block_variance: 10,
            rollup_start_block_number: 2,
            rollup_stop_block_number: 2,
        ))
        .named("get_genesis_info_1")
        .up_to_n_times(1) // We need to mount a new genesis info after restart
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("get_commitment_state_1")
        .up_to_n_times(1) // We need to mount a new commitment state after restart
        .mount()
        .await;

    test_conductor.mock_sequencer_genesis().mount().await;

    test_conductor.mock_abci_info(4).mount().await;

    test_conductor
        .mock_get_filtered_sequencer_block(3)
        .mount()
        .await;
    test_conductor
        .mock_get_filtered_sequencer_block(4)
        .mount()
        .await;

    let execute_block_1 = test_conductor
        .mock_execute_block(2, [2; 64], [1; 64])
        .expect(1)
        .named("execute_block_1")
        .mount_as_scoped()
        .await;

    let update_commitment_state_1 = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 2,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("update_commitment_state_1")
        .expect(1)
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_1.wait_until_satisfied(),
            update_commitment_state_1.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the first soft block and updated the first soft \
         commitment state within 1000ms",
    );

    test_conductor
        .mock_get_genesis_info(genesis_info!(
            sequencer_start_height: 4,
            celestia_block_variance: 10,
            rollup_start_block_number: 3,
            rollup_stop_block_number: 9,
        ))
        .named("get_genesis_info_2")
        .mount()
        .await;

    test_conductor
        .mock_get_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 2,
                hash: [2; 64],
                parent: [1; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("get_commitment_state_2")
        .mount()
        .await;

    let execute_block_2 = test_conductor
        .mock_execute_block(3, [3; 64], [2; 64])
        .named("execute_block_2")
        .expect(1)
        .mount_as_scoped()
        .await;

    let update_commitment_state_2 = test_conductor
        .mock_update_commitment_state(commitment_state!(
            firm: (
                number: 1,
                hash: [1; 64],
                parent: [0; 64],
            ),
            soft: (
                number: 3,
                hash: [3; 64],
                parent: [2; 64],
            ),
            base_celestia_height: 1,
        ))
        .named("update_commitment_state_2")
        .expect(1)
        .mount_as_scoped()
        .await;

    timeout(
        Duration::from_millis(1000),
        join(
            execute_block_2.wait_until_satisfied(),
            update_commitment_state_2.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the second soft block and updated the second soft \
         commitment state within 1000ms",
    );
}
