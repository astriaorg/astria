use std::time::Duration;

use astria_conductor::{
    config::CommitLevel,
    Conductor,
    Config,
};
use astria_core::generated::astria::execution::v1::{
    GetCommitmentStateRequest,
    GetGenesisInfoRequest,
};
use futures::future::{
    join,
    join4,
};
use serde_json::json;
use telemetry::metrics;
use tokio::time::{
    sleep,
    timeout,
};
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
    commitment_state,
    genesis_info,
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
    mount_executed_block,
    mount_get_commitment_state,
    mount_get_genesis_info,
    mount_sequencer_commit,
    mount_sequencer_genesis,
    mount_sequencer_validator_set,
    mount_update_commitment_state,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn simple() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
    );

    mount_get_commitment_state!(
        test_conductor,
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

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
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

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
    );

    mount_get_commitment_state!(
        test_conductor,
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

    let execute_block_number_2 = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state_number_2 = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    let execute_block_number_3 = mount_executed_block!(
        test_conductor,
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
    );

    let update_commitment_state_number_3 = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        soft: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        base_celestia_height: 1,
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

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 5,
            hash: [1; 64],
            parent: [0; 64],
        ),
        soft: (
            number: 5,
            hash: [1; 64],
            parent: [0; 64],
        ),
        base_celestia_height: 1,
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

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 6,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 6,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 6,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
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

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
    );

    mount_get_commitment_state!(
        test_conductor,
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
        base_celestia_height: 4,
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

    let execute_block = mount_executed_block!(
        test_conductor,
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
    );

    let update_commitment_state = mount_update_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 4,
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
        "get_genesis_info",
        matcher::message_type::<GetGenesisInfoRequest>(),
    )
    .respond_with(GrpcResponse::constant_response(
        genesis_info!(sequencer_start_block_height: 1,
            sequencer_stop_block_height: 10,
            celestia_block_variance: 10,
            rollup_start_block_height: 0,),
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
/// for that height before restarting and continuing after fetching new genesis info and commitment
/// state.
///
/// It consists of the following steps:
/// 1. Mount commitment state and genesis info with a stop height of 3 for the first height, only
///    responding up to 1 time so that the same info is not provided after conductor restart.
/// 2. Mount sequencer genesis and celestia header network head.
/// 3. Mount firm blocks for heights 3 and 4.
/// 4. Mount `execute_block` and `update_commitment_state` for firm block 3, expecting only one call
///    since they should not be called after restarting.
/// 5. Wait ample time for conductor to restart before performing the next set of mounts.
/// 6. Mount new genesis info and updated commitment state with rollup start block height of 2 to
///    reflect that the first block has already been executed.
/// 7. Mount `execute_block` and `update_commitment_state` for firm block 4, awaiting their
///    satisfaction.
#[expect(clippy::too_many_lines, reason = "All lines reasonably necessary")]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn conductor_restarts_after_reaching_stop_block_height() {
    let test_conductor = spawn_conductor(CommitLevel::FirmOnly).await;

    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 3,
        celestia_block_variance: 10,
        rollup_start_block_height: 0,
        up_to_n_times: 1, // Only respond once, since updated information is needed after restart.
    );

    mount_get_commitment_state!(
        test_conductor,
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
        up_to_n_times: 1,
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

    let execute_block_1 = mount_executed_block!(
        test_conductor,
        mock_name: "execute_block_1",
        number: 2,
        hash: [2; 64],
        parent: [1; 64],
        expected_calls: 1, // should not be called again upon restart
    );

    let update_commitment_state_1 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_1",
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
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

    // Wait for conductor to restart before performing the next set of mounts
    sleep(Duration::from_millis(1000)).await;

    // Mount new genesis info and commitment state with updated heights
    mount_get_genesis_info!(
        test_conductor,
        sequencer_start_block_height: 1,
        sequencer_stop_block_height: 10,
        celestia_block_variance: 10,
        rollup_start_block_height: 1,
    );

    mount_get_commitment_state!(
        test_conductor,
        firm: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        soft: (
            number: 2,
            hash: [2; 64],
            parent: [1; 64],
        ),
        base_celestia_height: 1,
    );

    let execute_block_2 = mount_executed_block!(
        test_conductor,
        mock_name: "execute_block_2",
        number: 3,
        hash: [3; 64],
        parent: [2; 64],
        expected_calls: 1,
    );

    let update_commitment_state_2 = mount_update_commitment_state!(
        test_conductor,
        mock_name: "update_commitment_state_2",
        firm: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        soft: (
            number: 3,
            hash: [3; 64],
            parent: [2; 64],
        ),
        base_celestia_height: 1,
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
