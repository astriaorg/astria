#[macro_export]
macro_rules! block {
    (number: $number:expr,hash: $hash:expr,parent: $parent:expr $(,)?) => {
        ::astria_core::generated::execution::v1alpha2::Block {
            number: $number,
            hash: ::bytes::Bytes::from(Vec::from($hash)),
            parent_block_hash: ::bytes::Bytes::from(Vec::from($parent)),
            timestamp: Some(::pbjson_types::Timestamp {
                seconds: 1,
                nanos: 1,
            }),
        }
    };
}

#[macro_export]
macro_rules! commitment_state {
    (
        firm: (number: $firm_number:expr,hash: $firm_hash:expr,parent: $firm_parent:expr $(,)?),
        soft: (number: $soft_number:expr,hash: $soft_hash:expr,parent: $soft_parent:expr $(,)?)
        $(,)?
    ) => {
       ::astria_core::generated::execution::v1alpha2::CommitmentState {
            firm: Some($crate::block!(
                number: $firm_number,
                hash: $firm_hash,
                parent: $firm_parent,
            )),
            soft: Some($crate::block!(
                number: $soft_number,
                hash: $soft_hash,
                parent: $soft_parent,
            )),
        }
    };
}

#[macro_export]
macro_rules! filtered_sequencer_block {
    (sequencer_height: $height:expr) => {{
        let block = ::astria_core::sequencer::v1::test_utils::ConfigureCometBftBlock {
            height: $height,
            rollup_transactions: vec![($crate::ROLLUP_ID, b"hello_world".to_vec())],
            ..Default::default()
        }
        .make();
        ::astria_core::sequencer::v1::SequencerBlock::try_from_cometbft(block)
            .unwrap()
            .into_filtered_block([$crate::ROLLUP_ID])
            .into_raw()
    }};
}

// XXX: We have to live with rustfmt mangling the pattern match. Fixing it triggers warnings:
// 1. applying #[rustfmt::skip] on the macro or on the containing module triggers issue 52234.
// 2. appylin #![rustfmt::skip] triggers issue 64266.
#[macro_export]
macro_rules! genesis_info {
    (
        sequencer_genesis_block_height:
        $sequencer_height:expr,celestia_base_block_height:
        $celestia_height:expr,celestia_block_variance:
        $variance:expr $(,)?
    ) => {
        ::astria_core::generated::execution::v1alpha2::GenesisInfo {
            rollup_id: ::bytes::Bytes::from($crate::ROLLUP_ID.to_vec()),
            sequencer_genesis_block_height: $sequencer_height,
            celestia_base_block_height: $celestia_height,
            celestia_block_variance: $variance,
        }
    };
}

#[macro_export]
macro_rules! mount_get_commitment_state {
    (
        $test_env:ident,
        firm: ( number: $firm_number:expr, hash: $firm_hash:expr, parent: $firm_parent:expr$(,)? ),
        soft: ( number: $soft_number:expr, hash: $soft_hash:expr, parent: $soft_parent:expr$(,)? )
        $(,)?
    ) => {
        $test_env
            .mount_get_commitment_state($crate::commitment_state!(
                firm: (
                    number: $firm_number,
                    hash: $firm_hash,
                    parent: $firm_parent,
                ),
                soft: (
                    number: $soft_number,
                    hash: $soft_hash,
                    parent: $soft_parent,
                ),
            ))
        .await
    };
}

#[macro_export]
macro_rules! mount_update_commitment_state {
    (
        $test_env:ident,
        firm: ( number: $firm_number:expr, hash: $firm_hash:expr, parent: $firm_parent:expr$(,)? ),
        soft: ( number: $soft_number:expr, hash: $soft_hash:expr, parent: $soft_parent:expr$(,)? )
        $(,)?
    ) => {
        $test_env
            .mount_update_commitment_state($crate::commitment_state!(
                firm: (
                    number: $firm_number,
                    hash: $firm_hash,
                    parent: $firm_parent,
                ),
                soft: (
                    number: $soft_number,
                    hash: $soft_hash,
                    parent: $soft_parent,
                ),
        ))
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
macro_rules! mount_executed_block {
    (
        $test_env:ident,
        number: $number:expr,
        hash: $hash:expr,
        parent: $parent:expr $(,)?
    ) => {{
        use ::base64::prelude::*;
        $test_env.mount_execute_block(
            ::serde_json::json!({
                "prev_block_hash": BASE64_STANDARD.encode($parent),
                "transactions": [{"sequenced_data": BASE64_STANDARD.encode(b"hello_world")}],
            }),
            $crate::block!(
                number: $number,
                hash: $hash,
                parent: $parent,
            )
        )
        .await
    }}
}

#[macro_export]
macro_rules! mount_get_filtered_sequencer_block {
    ($test_env:ident, sequencer_height: $height:expr $(,)?) => {
        $test_env
            .mount_get_filtered_sequencer_block(
                ::astria_core::generated::sequencer::v1::GetFilteredSequencerBlockRequest {
                    height: $height,
                    rollup_ids: vec![$crate::ROLLUP_ID.to_vec()],
                },
                $crate::filtered_sequencer_block!(sequencer_height: $height),
            )
            .await;
    };
}

#[macro_export]
macro_rules! mount_get_genesis_info {
    (
        $test_env:ident,
        sequencer_genesis_block_height: $sequencer_height:expr,
        celestia_base_block_height: $celestia_height:expr,
        celestia_block_variance: $variance:expr
        $(,)?
    ) => {
        $test_env.mount_get_genesis_info(
            $crate::genesis_info!(
                sequencer_genesis_block_height: $sequencer_height,
                celestia_base_block_height: $celestia_height,
                celestia_block_variance: $variance,
            )
        ).await;
    };
}

// fn latest_commit() -> response::Wrapper<tendermint_rpc::endpoint::commit::Response> {
//     use sequencer_client::tendermint::{
//         account,
//         block::{
//             header::{
//                 Header,
//                 Version,
//             },
//             signed_header::SignedHeader,
//             Commit,
//         },
//         chain,
//         hash::{
//             AppHash,
//             Hash,
//         },
//         time::Time,
//     };
//     let response = tendermint_rpc::endpoint::commit::Response {
//         signed_header: SignedHeader::new(
//             Header {
//                 version: Version {
//                     block: 1,
//                     app: 1,
//                 },
//                 chain_id: "mocksequencer-1000".parse::<chain::Id>().unwrap(),
//                 height: 1u32.into(),
//                 time: Time::now(),
//                 last_block_id: None,
//                 last_commit_hash: None,
//                 data_hash: None,
//                 validators_hash: Hash::Sha256([0u8; 32]),
//                 next_validators_hash: Hash::Sha256([0u8; 32]),
//                 consensus_hash: Hash::Sha256([0u8; 32]),
//                 app_hash: AppHash::try_from(vec![0u8; 32]).unwrap(),
//                 last_results_hash: None,
//                 evidence_hash: None,
//                 proposer_address: account::Id::new([0u8; 20]),
//             },
//             Commit {
//                 height: 1u32.into(),
//                 ..Commit::default()
//             },
//         )
//         .unwrap(),
//         canonical: true,
//     };
//     response::Wrapper::new_with_id(tendermint_rpc::Id::uuid_v4(), Some(response), None)
// }
