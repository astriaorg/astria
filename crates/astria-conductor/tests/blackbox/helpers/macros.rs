#[macro_export]
macro_rules! celestia_network_head {
    (height: $height:expr $(,)?) => {
        celestia_network_head!(height: $height, chain_id: $crate::helpers::CELESTIA_CHAIN_ID)
    };
    (height: $height:expr,chain_id: $chain_id:expr $(,)?) => {
        ::celestia_types::ExtendedHeader {
            header: ::celestia_tendermint::block::header::Header {
                height: $height.into(),
                version: ::celestia_tendermint::block::header::Version {
                    block: 0,
                    app: 0,
                },
                chain_id: $chain_id.try_into().unwrap(),
                time: ::celestia_tendermint::Time::from_unix_timestamp(1, 1).unwrap(),
                last_block_id: None,
                last_commit_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                data_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                validators_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                next_validators_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                consensus_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                app_hash: vec![0; 32].try_into().unwrap(),
                last_results_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                evidence_hash: ::celestia_tendermint::Hash::Sha256([0; 32]),
                proposer_address: vec![0u8; 20].try_into().unwrap(),
            },
            commit: ::celestia_tendermint::block::Commit {
                height: $height.into(),
                ..Default::default()
            },
            validator_set: ::celestia_tendermint::validator::Set::without_proposer(vec![]),
            dah: ::celestia_types::DataAvailabilityHeader {
                row_roots: vec![],
                column_roots: vec![],
            },
        }
    };
}

#[macro_export]
macro_rules! commitment_state {
    (
        firm:
        (number: $firm_number:expr,hash: $firm_hash:expr,parent: $firm_parent:expr $(,)?),soft:
        (number: $soft_number:expr,hash: $soft_hash:expr,parent: $soft_parent:expr $(,)?),base_celestia_height:
        $base_celestia_height:expr $(,)?
    ) => {
        ::astria_core::generated::astria::execution::v2::CommitmentState {
            firm: Some($crate::helpers::make_block(
                $firm_number,
                $firm_hash,
                $firm_parent,
            )),
            soft: Some($crate::helpers::make_block(
                $soft_number,
                $soft_hash,
                $soft_parent,
            )),
            base_celestia_height: $base_celestia_height,
        }
    };
}

// XXX: We have to live with rustfmt mangling the pattern match. Fixing it triggers warnings:
// 1. applying #[rustfmt::skip] on the macro or on the containing module triggers issue 52234.
// 2. applying #![rustfmt::skip] triggers issue 64266.
#[macro_export]
macro_rules! genesis_info {
    (
        sequencer_start_height:
        $start_height:expr,celestia_block_variance:
        $variance:expr,rollup_start_block_number:
        $rollup_start_block_number:expr, rollup_stop_block_number:
        $rollup_stop_block_number:expr $(,)?
    ) => {
        genesis_info!(
            sequencer_start_height: $start_height,
            celestia_block_variance: $variance,
            rollup_start_block_number: $rollup_start_block_number,
            rollup_stop_block_number: $rollup_stop_block_number,
            halt_at_rollup_stop_number: false,
        )
    };
    (
        sequencer_start_height:
        $start_height:expr,celestia_block_variance:
        $variance:expr,rollup_start_block_number:
        $rollup_start_block_number:expr,
        rollup_stop_block_number: $rollup_stop_block_number:expr,
        halt_at_rollup_stop_number: $halt_at_rollup_stop_number:expr $(,)?
    ) => {
        ::astria_core::generated::astria::execution::v2::GenesisInfo {
            rollup_id: Some($crate::ROLLUP_ID.to_raw()),
            sequencer_start_height: $start_height,
            celestia_block_variance: $variance,
            rollup_start_block_number: $rollup_start_block_number,
            rollup_stop_block_number: $rollup_stop_block_number,
            sequencer_chain_id: $crate::SEQUENCER_CHAIN_ID.to_string(),
            celestia_chain_id: $crate::helpers::CELESTIA_CHAIN_ID.to_string(),
            halt_at_rollup_stop_number: $halt_at_rollup_stop_number,
        }
    };
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
