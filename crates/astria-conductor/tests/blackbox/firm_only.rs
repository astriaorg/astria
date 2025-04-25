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
use serde_json::json;
use telemetry::metrics;
use tokio::time::timeout;
use wiremock::{
    matchers::{
        body_partial_json,
        header,
    },
    Mock,
    ResponseTemplate,
};

use crate::{
    celestia_network_head,
    execution_session,
    helpers::{
        make_config,
        spawn_conductor,
        MockGrpc,
        CELESTIA_BEARER_TOKEN,
        CELESTIA_CHAIN_ID,
        SEQUENCER_CHAIN_ID,
    },
    mount_celestia_blobs,
    mount_celestia_header_network_head,
    mount_create_execution_session,
    mount_execute_block,
    mount_sequencer_commit,
    mount_sequencer_genesis,
    mount_sequencer_validator_set,
    mount_update_commitment_state,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

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

    mount_celestia_header_network_head!(
        test_conductor,
        height: 1u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
    );

    timeout(
        Duration::from_millis(2000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the firm block and updated the firm commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn submits_two_heights_in_succession() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

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
        ),
    );

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 2u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    mount_sequencer_commit!(
        test_conductor,
        height: 4u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 3u32);

    let execute_block_number_2 = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state_number_2 = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
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
        number: 3,
        hash: "3",
        parent: "2",
    );

    let update_commitment_state_number_3 = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        soft: (
            number: 3,
            hash: "3",
            parent: "2",
        ),
        lowest_celestia_search_height: 1,
    );

    timeout(
        Duration::from_millis(2000),
        join4(
            execute_block_number_2.wait_until_satisfied(),
            update_commitment_state_number_2.wait_until_satisfied(),
            execute_block_number_3.wait_until_satisfied(),
            update_commitment_state_number_3.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the firm block and updated the firm commitment state \
         within 2000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn skips_already_executed_heights() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

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
                number: 5,
                hash: "1",
                parent: "0",
            ),
            soft: (
                number: 5,
                hash: "1",
                parent: "0",
            ),
            lowest_celestia_search_height: 1,
        ),
    );

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 2u32,
    );

    // The blob contains sequencer heights 6 and 7, but no commits or validator sets are mounted.
    // XXX: A non-fetch cannot be tested for programmatically right now. Running the test with
    // tracing enabled should show that the sequencer metadata at height 6 is explicitly
    // skipped.
    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [6, 7],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 7u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 6u32);

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 6,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 6,
            hash: "2",
            parent: "1",
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
        "conductor should have executed the firm block and updated the firm commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn fetch_from_later_celestia_height() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

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
            lowest_celestia_search_height: 4,
        ),
    );

    mount_sequencer_genesis!(test_conductor);

    mount_celestia_header_network_head!(
        test_conductor,
        height: 5u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 4,
        sequencer_heights: [3],
    );

    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );

    mount_sequencer_validator_set!(test_conductor, height: 2u32);

    let execute_block = mount_execute_block!(
        test_conductor,
        number: 2,
        hash: "2",
        parent: "1",
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 4,
    );

    timeout(
        Duration::from_millis(2000),
        join(
            execute_block.wait_until_satisfied(),
            update_commitment_state.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the firm block and updated the firm commitment state \
         within 1000ms",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn exits_on_celestia_chain_id_mismatch() {
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
        execution_commit_level: CommitLevel::FirmOnly,
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
        )
    )))
    .expect(0..)
    .mount(&mock_grpc.mock_server)
    .await;

    let bad_chain_id = "bad_chain_id";

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
        "result": celestia_network_head!(height: 1u32, chain_id: bad_chain_id),
    })))
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
                        "expected Celestia chain id `{CELESTIA_CHAIN_ID}` does not match actual: \
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

/// Tests that the conductor correctly stops at the stop block height and executes the firm block
/// for that height before restarting and continuing after requesting new execution session.
///
/// It consists of the following steps:
/// 1. Mount execution session with a stop number of 2 for the first height (sequencer height 3),
///    only responding up to 1 time so that the same info is not provided after conductor restart.
/// 2. Mount sequencer genesis and celestia header network head.
/// 3. Mount firm blocks for heights 3 and 4.
/// 4. Mount `execute_block` and `update_commitment_state` for firm block 3, expecting only one call
///    since they should not be called after restarting.
/// 5. Wait ample time for conductor to restart before performing the next set of mounts.
/// 6. Mount new execution session with rollup start block number of 3 to reflect that the first
///    block has already been executed.
/// 7. Mount `execute_block` and `update_commitment_state` for firm block 4, awaiting their
///    satisfaction.
#[expect(clippy::too_many_lines, reason = "All lines reasonably necessary")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn restarts_after_reaching_stop_block_height() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

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
        up_to_n_times: 1, // Only respond once, since a new execution session is needed after restart.
    );

    mount_sequencer_genesis!(test_conductor);
    mount_celestia_header_network_head!(
        test_conductor,
        height: 2u32,
    );

    mount_celestia_blobs!(
        test_conductor,
        celestia_height: 1,
        sequencer_heights: [3, 4],
    );
    mount_sequencer_commit!(
        test_conductor,
        height: 3u32,
    );
    mount_sequencer_commit!(
        test_conductor,
        height: 4u32,
    );
    mount_sequencer_validator_set!(test_conductor, height: 2u32);
    mount_sequencer_validator_set!(test_conductor, height: 3u32);

    let execute_block_1 = mount_execute_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: "2",
        parent: "1",
        expected_calls: 1, // should not be called again upon restart
    );

    let update_commitment_state_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_1",
        firm: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        soft: (
            number: 2,
            hash: "2",
            parent: "1",
        ),
        lowest_celestia_search_height: 1,
        expected_calls: 1, // should not be called again upon restart
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
        "conductor should have executed the first firm block and updated the first firm \
         commitment state twice within 1000ms",
    );

    // Mount new execution session with updated heights and commitment state.
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
                number: 2,
                hash: "2",
                parent: "1",
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
            number: 3,
            hash: "3",
            parent: "2",
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
        Duration::from_millis(2000),
        join(
            execute_block_2.wait_until_satisfied(),
            update_commitment_state_2.wait_until_satisfied(),
        ),
    )
    .await
    .expect(
        "conductor should have executed the second firm block and updated the second firm \
         commitment state twice within 2000ms",
    );
}
