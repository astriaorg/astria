use std::time::Duration;

use astria_conductor::{
    config::CommitLevel,
    Conductor,
    Config,
};
use astria_core::generated::astria::execution::v2::CreateExecutionSessionRequest;
use futures::future::{
    join,
    join4,
};
use telemetry::metrics;
use tokio::time::timeout;

use crate::{
    execution_session,
    helpers::{
        make_config,
        mount_genesis,
        spawn_conductor,
        MockGrpc,
    },
    mount_abci_info,
    mount_create_execution_session,
    mount_execute_block,
    mount_get_filtered_sequencer_block,
    mount_sequencer_genesis,
    mount_update_commitment_state,
    SEQUENCER_CHAIN_ID,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
    );

    mount_sequencer_genesis!(test_conductor);

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
    );

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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
    );

    mount_sequencer_genesis!(test_conductor);

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    let execute_block_number_2 = mount_execute_block!(
        test_conductor,
        mock_name: "first_execute",
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state_number_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "first_update",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
    );

    let execute_block_number_3 = mount_execute_block!(
        test_conductor,
        mock_name: "second_execute",
        number: 3,
        hash: "3",
        parent: "2",
    );

    let update_commitment_state_number_3 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "second_update",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
    );

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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 5,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
    );

    mount_sequencer_genesis!(test_conductor);

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 7,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 7,
    );

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 6,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 6,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
    );

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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 10,
            sequencer_start_block_height: 12,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        )
    );

    mount_sequencer_genesis!(test_conductor);

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 12,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 12,
    );

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1
    );

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
        "create_execution_session",
        matcher::message_type::<CreateExecutionSessionRequest>(),
    )
    .respond_with(GrpcResponse::constant_response(execution_session!(
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
    )))
    .expect(0..)
    .mount(&mock_grpc.mock_server)
    .await;

    let bad_chain_id = "bad_chain_id";
    mount_genesis(&mock_http, bad_chain_id).await;

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
/// 1. Mount execution session with a rollup stop number of 2 (sequencer height 3), responding only
///    up to 1 time so that the same information is not retrieved after restarting.
/// 2. Mount sequencer genesis, ABCI info, and sequencer blocks for heights 3 and 4.
/// 3. Mount `execute_block` and `update_commitment_state` mocks for the soft block at height 3,
///    expecting only 1 call and timing out after 1000ms.
/// 4. Mount updated execution session with a rollup stop number of 9 (more than high enough) and a
///    start block number of 2, reflecting that the first block has already been executed.
/// 5. Mount `execute_block` and `update_commitment_state` mocks for the soft block at height 4,
///    awaiting their satisfaction.
#[expect(
    clippy::too_many_lines,
    reason = "All lines reasonably necessary for the thoroughness of this test"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restarts_after_reaching_stop_block_height() {
    let test_conductor = spawn_conductor(CommitLevel::SoftOnly).await;

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 2,
            rollup_end_block_number: 2,
            sequencer_start_block_height: 3,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
        up_to_n_times: 1, // We need a new execution session after restart
    );

    mount_sequencer_genesis!(test_conductor);

    mount_abci_info!(
        test_conductor,
        latest_sequencer_height: 4,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 3,
    );

    mount_get_filtered_sequencer_block!(
        test_conductor,
        sequencer_height: 4,
    );

    let execute_block_1 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: "2",
        parent: "1",
        expected_calls: 1,
    );

    let update_commitment_state_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_1",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

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

    mount_create_execution_session!(
        test_conductor,
        execution_session_parameters: (
            rollup_start_block_number: 3,
            rollup_end_block_number: 9,
            sequencer_start_block_height: 4,
            celestia_max_look_ahead: 10,
        ),
        commitment_state: (
            firm: (
                number: 1,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 2,
                hash: "2",
                parent: "1",
            ),
            lowest_celestia_search_height: 1,
        ),
    );

    let execute_block_2 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_2",
        number: 3,
        hash: "3",
        parent: "2",
        expected_calls: 1,
    );

    let update_commitment_state_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_2",
        firm: (
            number: 1,
            hash: "1",
            parent: "0",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1,
    );

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
