#[macro_export]
macro_rules! block_metadata {
    (number: $number:expr,hash: $hash:expr,parent: $parent:expr $(,)?) => {
        ::astria_core::generated::astria::execution::v2::ExecutedBlockMetadata {
            number: $number,
            hash: $hash.to_string(),
            parent_hash: $parent.to_string(),
            timestamp: Some(::pbjson_types::Timestamp {
                seconds: 1,
                nanos: 1,
            }),
            sequencer_block_hash: String::new(),
        }
    };
}

#[macro_export]
macro_rules! celestia_network_head {
    (height: $height:expr) => {
        celestia_network_head!(height: $height, chain_id: $crate::helpers::CELESTIA_CHAIN_ID)
    };
    (height: $height:expr,chain_id: $chain_id:expr $(,)?) => {
        ::celestia_types::ExtendedHeader {
            header: ::tendermint::block::header::Header {
                height: $height.into(),
                version: ::tendermint::block::header::Version {
                    block: 0,
                    app: 0,
                },
                chain_id: $chain_id.try_into().unwrap(),
                time: ::tendermint::Time::from_unix_timestamp(1, 1).unwrap(),
                last_block_id: None,
                last_commit_hash: ::tendermint::Hash::Sha256([0; 32]).into(),
                data_hash: ::tendermint::Hash::Sha256([0; 32]).into(),
                validators_hash: ::tendermint::Hash::Sha256([0; 32]),
                next_validators_hash: ::tendermint::Hash::Sha256([0; 32]),
                consensus_hash: ::tendermint::Hash::Sha256([0; 32]),
                app_hash: vec![0; 32].try_into().unwrap(),
                last_results_hash: ::tendermint::Hash::Sha256([0; 32]).into(),
                evidence_hash: ::tendermint::Hash::Sha256([0; 32]).into(),
                proposer_address: vec![0u8; 20].try_into().unwrap(),
            },
            commit: ::tendermint::block::Commit {
                height: $height.into(),
                ..Default::default()
            },
            validator_set: ::tendermint::validator::Set::without_proposer(vec![]),
            dah: ::celestia_types::DataAvailabilityHeader::new_unchecked(vec![],vec![],),
        }
    };
}

#[macro_export]
macro_rules! filtered_sequencer_block {
    (sequencer_height: $height:expr) => {{
        let block = ::astria_core::protocol::test_utils::ConfigureSequencerBlock {
            height: $height,
            sequence_data: vec![($crate::ROLLUP_ID, $crate::helpers::data())],
            ..Default::default()
        }
        .make();
        block.into_filtered_block([$crate::ROLLUP_ID]).into_raw()
    }};
}

// XXX: We have to live with rustfmt mangling the pattern match. Fixing it triggers warnings:
// 1. applying #[rustfmt::skip] on the macro or on the containing module triggers issue 52234.
// 2. applying #![rustfmt::skip] triggers issue 64266.
#[macro_export]
macro_rules! execution_session_parameters {
    (
        rollup_start_block_number:
        $rollup_start_block_number:expr,rollup_end_block_number:
        $rollup_end_block_number:expr,sequencer_start_block_height:
        $start_height:expr,celestia_max_look_ahead:
        $celestia_max_look_ahead:expr $(,)?
    ) => {
        ::astria_core::generated::astria::execution::v2::ExecutionSessionParameters {
            rollup_id: Some($crate::ROLLUP_ID.to_raw()),
            rollup_start_block_number: $rollup_start_block_number,
            rollup_end_block_number: $rollup_end_block_number,
            sequencer_start_block_height: $start_height,
            sequencer_chain_id: $crate::SEQUENCER_CHAIN_ID.to_string(),
            celestia_chain_id: $crate::helpers::CELESTIA_CHAIN_ID.to_string(),
            celestia_search_height_max_look_ahead: $celestia_max_look_ahead,
        }
    };
}

#[macro_export]
macro_rules! signed_header {
    (height: $height:expr,block_hash: $block_hash:expr $(,)?) => {};
}

#[macro_export]
macro_rules! mount_celestia_blobs {
    (
        $test_env:ident,
        celestia_height: $celestia_height:expr,
        sequencer_heights: [ $($sequencer_height:expr),+ ]
        $(,)?
    ) => {
        mount_celestia_blobs!(
            $test_env,
            celestia_height: $celestia_height,
            sequencer_heights: [ $($sequencer_height),+ ],
            delay: None,
        )
    };
    (
        $test_env:ident,
        celestia_height: $celestia_height:expr,
        sequencer_heights: [ $($sequencer_height:expr),+ ],
        delay: $delay:expr
        $(,)?
    ) => {{
        let blobs = $crate::helpers::make_blobs(&[ $( $sequencer_height ),+ ]);
        $test_env
            .mount_celestia_blob_get_all(
                $celestia_height,
                $crate::sequencer_namespace(),
                vec![blobs.header],
                $delay,
            )
            .await;
        $test_env
            .mount_celestia_blob_get_all(
                $celestia_height,
                $crate::rollup_namespace(),
                vec![blobs.rollup],
                $delay,
            )
            .await
    }};
}

#[macro_export]
macro_rules! mount_celestia_header_network_head {
    (
        $test_env:ident,
        height: $height:expr $(,)?
    ) => {
        $test_env
            .mount_celestia_header_network_head(
                $crate::celestia_network_head!(height: $height, chain_id: $crate::helpers::CELESTIA_CHAIN_ID),
            )
            .await;
    }
}

#[macro_export]
macro_rules! mount_soft_update_commitment_state {
    (
        $test_env:ident,
        number: $number:expr,
        lowest_celestia_search_height: $lowest_celestia_search_height:expr
        $(,)?
    ) => {
        mount_soft_update_commitment_state!(
            $test_env,
            mock_name: None,
            number: $number,
            lowest_celestia_search_height: $lowest_celestia_search_height,
            expected_calls: 1,
        )
    };
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
        lowest_celestia_search_height: $lowest_celestia_search_height:expr
        $(,)?
    ) => {
        mount_soft_update_commitment_state!(
            $test_env,
            mock_name: $mock_name,
            number: $number,
            lowest_celestia_search_height: $lowest_celestia_search_height,
            expected_calls: 1,
        )
    };
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
        lowest_celestia_search_height: $lowest_celestia_search_height:expr,
        expected_calls: $expected_calls:expr
        $(,)?
    ) => {
        $test_env
            .mount_update_commitment_state(
                $mock_name.into(),
                $number,
                false,
                $lowest_celestia_search_height,
                $expected_calls,
        )
        .await
    };
}

#[macro_export]
macro_rules! mount_firm_update_commitment_state {
    (
        $test_env:ident,
        number: $number:expr,
        lowest_celestia_search_height: $lowest_celestia_search_height:expr
        $(,)?
    ) => {
        mount_firm_update_commitment_state!(
            $test_env,
            mock_name: None,
            number: $number,
            lowest_celestia_search_height: $lowest_celestia_search_height,
            expected_calls: 1,
        )
    };
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
        lowest_celestia_search_height: $lowest_celestia_search_height:expr,
        expected_calls: $expected_calls:expr
        $(,)?
    ) => {
        $test_env
            .mount_update_commitment_state(
                $mock_name.into(),
                $number,
                true,
                $lowest_celestia_search_height,
                $expected_calls,
        )
        .await
    };
}

#[macro_export]
macro_rules! mount_abci_info {
    ($test_env:ident,latest_sequencer_height: $height:expr $(,)?) => {
        $test_env.mount_abci_info($height).await;
    };
}

#[macro_export]
macro_rules! mount_execute_block {
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
        expected_calls: $expected_calls:expr $(,)?
    ) => {{
        $test_env.mount_execute_block(
            $mock_name.into(),
            $number,
            $expected_calls,
        )
        .await
    }};
    (
        $test_env:ident,
        mock_name: $mock_name:expr,
        number: $number:expr,
    ) => {
        mount_execute_block!(
            $test_env,
            mock_name: None,
            number: $number,
            expected_calls: 1,
        )
    };
    (
        $test_env:ident,
        number: $number:expr,
    ) => {
        mount_execute_block!(
            $test_env,
            mock_name: None,
            number: $number,
        )
    };
}

#[macro_export]
macro_rules! mount_get_filtered_sequencer_block {
    ($test_env:ident, sequencer_height: $height:expr, delay: $delay:expr $(,)?) => {
        $test_env
            .mount_get_filtered_sequencer_block(
                ::astria_core::generated::astria::sequencerblock::v1::GetFilteredSequencerBlockRequest {
                    height: $height,
                    rollup_ids: vec![$crate::ROLLUP_ID.to_raw()],
                },
                $crate::filtered_sequencer_block!(sequencer_height: $height),
                $delay,
            )
            .await;
    };
    ($test_env:ident, sequencer_height: $height:expr$(,)?) => {
        mount_get_filtered_sequencer_block!(
            $test_env,
            sequencer_height: $height,
            delay: Duration::from_secs(0),
        )
    };
}

#[macro_export]
macro_rules! mount_create_execution_session {
    (
        $test_env:ident,
        execution_session_parameters: (
            rollup_start_block_number: $rollup_start_block_number:expr,
            rollup_end_block_number: $rollup_end_block_number:expr,
            sequencer_start_block_height: $start_height:expr,
            celestia_max_look_ahead: $celestia_max_look_ahead:expr $(,)?
        ),
        commitment_state: (
            firm_number: $firm_number:expr,
            soft_number: $soft_number:expr,
            lowest_celestia_search_height: $lowest_celestia_search_height:expr$(,)?
        )
        $(,)?
    ) => {
        mount_create_execution_session!(
            $test_env,
            execution_session_parameters: (
                rollup_start_block_number: $rollup_start_block_number,
                rollup_end_block_number: $rollup_end_block_number,
                sequencer_start_block_height: $start_height,
                celestia_max_look_ahead: $celestia_max_look_ahead,
            ),
            commitment_state: (
                firm_number: $firm_number,
                soft_number: $soft_number,
                lowest_celestia_search_height: $lowest_celestia_search_height,
            ),
            expected_calls: 1,
            up_to_n_times: 1,
        )
    };
    (
        $test_env:ident,
        execution_session_parameters: (
            rollup_start_block_number: $rollup_start_block_number:expr,
            rollup_end_block_number: $rollup_end_block_number:expr,
            sequencer_start_block_height: $start_height:expr,
            celestia_max_look_ahead: $celestia_max_look_ahead:expr $(,)?
        ),
        commitment_state: (
            firm_number: $firm_number:expr,
            soft_number: $soft_number:expr,
            lowest_celestia_search_height: $lowest_celestia_search_height:expr$(,)?
        ),
        up_to_n_times: $up_to_n_times:expr
        $(,)?
    ) => {
        mount_create_execution_session!(
            $test_env,
            execution_session_parameters: (
                rollup_start_block_number: $rollup_start_block_number,
                rollup_end_block_number: $rollup_end_block_number,
                sequencer_start_block_height: $start_height,
                celestia_max_look_ahead: $celestia_max_look_ahead,
            ),
            commitment_state: (
                firm_number: $firm_number,
                soft_number: $soft_number,
                lowest_celestia_search_height: $lowest_celestia_search_height,
            ),
            expected_calls: 1,
            up_to_n_times: $up_to_n_times,
        )
    };
    (
        $test_env:ident,
        execution_session_parameters: (
            rollup_start_block_number: $rollup_start_block_number:expr,
            rollup_end_block_number: $rollup_end_block_number:expr,
            sequencer_start_block_height: $start_height:expr,
            celestia_max_look_ahead: $celestia_max_look_ahead:expr $(,)?
        ),
        commitment_state: (
            firm_number: $firm_number:expr,
            soft_number: $soft_number:expr,
            lowest_celestia_search_height: $lowest_celestia_search_height:expr$(,)?
        ),
        expected_calls: $expected_calls:expr,
        up_to_n_times: $up_to_n_times:expr $(,)?
    ) => {
        $test_env.mount_create_execution_session(
            $crate::execution_session_parameters!(
                rollup_start_block_number: $rollup_start_block_number,
                rollup_end_block_number: $rollup_end_block_number,
                sequencer_start_block_height: $start_height,
                celestia_max_look_ahead: $celestia_max_look_ahead,
            ),
            $firm_number,
            $soft_number,
            $lowest_celestia_search_height,
            $up_to_n_times,
            $expected_calls,
        ).await;
    };
}

#[macro_export]
macro_rules! mount_sequencer_commit {
    ($test_env:ident,height: $height:expr $(,)?) => {
        $test_env
            .mount_commit($crate::helpers::make_signed_header($height))
            .await;
    };
}

#[macro_export]
macro_rules! mount_sequencer_validator_set {
    ($test_env:ident,height: $height:expr $(,)?) => {
        $test_env
            .mount_validator_set($crate::helpers::make_validator_set($height))
            .await
    };
}

#[macro_export]
macro_rules! mount_sequencer_genesis {
    ($test_env:ident) => {
        $test_env.mount_genesis(SEQUENCER_CHAIN_ID).await;
    };
}

#[macro_export]
macro_rules! mount_get_executed_block_metadata {
    ($test_env:ident,number: $number:expr,) => {{
        $test_env.mount_get_executed_block_metadata($number).await
    }};
}

#[macro_export]
macro_rules! mount_execute_block_tonic_code {
    (
        $test_env:ident,
        parent: $parent:expr,
        status_code: $status_code:expr $(,)?
    ) => {{
        use ::base64::prelude::*;
        $test_env.mount_tonic_status_code(
            ::serde_json::json!({
                "sessionId": $crate::helpers::EXECUTION_SESSION_ID,
                "parentHash": $parent,
                "transactions": [
                    {"priceFeedData": {}},
                    {"sequencedData": BASE64_STANDARD.encode($crate::helpers::data())}
                ],
            }),
            $status_code
        ).await
    }};
}
