use std::{
    fmt::Write as _,
    time::Duration,
};

use astria_core::{
    generated::astria::composer::v1::{
        grpc_collector_service_client::GrpcCollectorServiceClient,
        SubmitRollupTransactionRequest,
    },
    primitive::v1::{
        RollupId,
        ROLLUP_ID_LEN,
    },
    protocol::transaction::v1::action::RollupDataSubmission,
};
use futures::future::join;
use tokio::time;

use crate::helper::{
    mount_broadcast_tx_sync_rollup_data_submissions_mock,
    signed_tx_from_request,
    spawn_composer,
    TEST_CHAIN_ID,
};

/// Test to check that the executor sends a signed transaction to the sequencer after its
/// `block_timer` has ticked
#[tokio::test]
async fn bundle_triggered_by_block_timer() {
    let test_composer = spawn_composer(&["test1"], None, true).await;
    let mut composer_client = GrpcCollectorServiceClient::connect(format!(
        "http://{}",
        test_composer.grpc_collector_addr
    ))
    .await
    .unwrap();

    let response_guard =
        mount_broadcast_tx_sync_rollup_data_submissions_mock(&test_composer.sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let rollup_id = RollupId::new([0; ROLLUP_ID_LEN]);
    let data = vec![0u8; 1000];

    let seq0 = RollupDataSubmission {
        data: data.clone().into(),
        rollup_id,
        fee_asset: "nria".parse().unwrap(),
    };

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    let submission_timeout =
        Duration::from_millis(test_composer.cfg.block_time_ms.saturating_add(100));
    time::timeout(submission_timeout, async {
        composer_client
            .submit_rollup_transaction(SubmitRollupTransactionRequest {
                rollup_id: Some(rollup_id.into_raw()),
                data: data.into(),
            })
            .await
            .expect("rollup transactions should have been submitted successfully to grpc collector")
    })
    .await
    .unwrap();
    time::advance(Duration::from_millis(test_composer.cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        response_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_rollup_data_submissions = [seq0];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(
        actions.len(),
        expected_rollup_data_submissions.len(),
        "received more than one action, one was supposed to fill the bundle"
    );

    for (action, expected_rollup_data_submission) in
        actions.iter().zip(expected_rollup_data_submissions.iter())
    {
        let rollup_data_submission = action.as_rollup_data_submission().unwrap();
        assert_eq!(
            rollup_data_submission.rollup_id, expected_rollup_data_submission.rollup_id,
            "chain id does not match. actual {:?} expected {:?}",
            rollup_data_submission.rollup_id, expected_rollup_data_submission.rollup_id
        );
        assert_eq!(
            rollup_data_submission.data, expected_rollup_data_submission.data,
            "data does not match expected data for action with rollup_id {:?}",
            rollup_data_submission.rollup_id,
        );
    }
}

/// Test to check that the executor sends a signed transaction with two sequence actions to the
/// sequencer.
#[tokio::test]
async fn two_rollup_data_submissions_single_bundle() {
    let test_composer = spawn_composer(&["test1"], None, true).await;
    let composer_client = GrpcCollectorServiceClient::connect(format!(
        "http://{}",
        test_composer.grpc_collector_addr
    ))
    .await
    .unwrap();

    let response_guard =
        mount_broadcast_tx_sync_rollup_data_submissions_mock(&test_composer.sequencer).await;

    // send two sequence actions to the executor, both small enough to fit in a single bundle
    // without filling it
    let seq0 = RollupDataSubmission {
        rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
        data: vec![0u8; 1000].into(),
        fee_asset: "nria".parse().unwrap(),
    };

    let seq1 = RollupDataSubmission {
        rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
        data: vec![1u8; 1000].into(),
        fee_asset: "nria".parse().unwrap(),
    };

    // Submit transactions concurrently so that the block timer does not tick between them and they
    // are bundled
    let submit_fut_1 = {
        let mut client = composer_client.clone();
        let seq0 = seq0.clone();
        async move {
            client
                .submit_rollup_transaction(SubmitRollupTransactionRequest {
                    rollup_id: Some(seq0.rollup_id.into_raw()),
                    data: seq0.data.clone(),
                })
                .await
                .unwrap()
        }
    };
    let submit_fut_2 = {
        let mut client = composer_client.clone();
        let seq1 = seq1.clone();
        async move {
            client
                .submit_rollup_transaction(SubmitRollupTransactionRequest {
                    rollup_id: Some(seq1.rollup_id.into_raw()),
                    data: seq1.data.clone(),
                })
                .await
                .unwrap()
        }
    };

    // make sure at least one block has passed so that the executor will submit the bundle
    // despite it not being full
    time::pause();
    let submission_timeout =
        Duration::from_millis(test_composer.cfg.block_time_ms.saturating_add(100));
    time::timeout(submission_timeout, join(submit_fut_1, submit_fut_2))
        .await
        .unwrap();
    time::advance(Duration::from_millis(test_composer.cfg.block_time_ms)).await;
    time::resume();

    // wait for the mock sequencer to receive the signed transaction
    tokio::time::timeout(
        Duration::from_millis(100),
        response_guard.wait_until_satisfied(),
    )
    .await
    .unwrap();

    // verify only one signed transaction was received by the mock sequencer
    let expected_rollup_data_submissions = [seq0, seq1];
    let requests = response_guard.received_requests().await;
    assert_eq!(requests.len(), 1);

    // verify the expected sequence actions were received
    let signed_tx = signed_tx_from_request(&requests[0]);
    let actions = signed_tx.actions();

    assert_eq!(
        actions.len(),
        expected_rollup_data_submissions.len(),
        "received more than one action, one was supposed to fill the bundle"
    );

    for (action, expected_rollup_data_submission) in
        actions.iter().zip(expected_rollup_data_submissions.iter())
    {
        let rollup_data_submission = action.as_rollup_data_submission().unwrap();
        assert_eq!(
            rollup_data_submission.rollup_id, expected_rollup_data_submission.rollup_id,
            "chain id does not match. actual {:?} expected {:?}",
            rollup_data_submission.rollup_id, expected_rollup_data_submission.rollup_id
        );
        assert_eq!(
            rollup_data_submission.data, expected_rollup_data_submission.data,
            "data does not match expected data for action with rollup_id {:?}",
            rollup_data_submission.rollup_id,
        );
    }
}

/// Test to check that executor's chain ID check is properly checked against the sequencer's chain
/// ID
#[tokio::test]
async fn chain_id_mismatch_returns_error() {
    let bad_chain_id = "bad_id";
    let test_composer = spawn_composer(&["test1"], Some(bad_chain_id), false).await;
    let expected_err_msg =
        format!("expected chain ID `{TEST_CHAIN_ID}`, but received `{bad_chain_id}`");
    let err = test_composer.composer.await.unwrap().unwrap_err();
    for cause in err.chain() {
        if cause.to_string().contains(&expected_err_msg) {
            return;
        }
    }
    let mut panic_msg = String::new();
    writeln!(
        &mut panic_msg,
        "did not find expected executor error message"
    )
    .unwrap();
    writeln!(&mut panic_msg, "expected cause:\n\t{expected_err_msg}").unwrap();
    writeln!(&mut panic_msg, "actual cause chain:\n\t{err:?}").unwrap();
    panic!("{panic_msg}");
}
