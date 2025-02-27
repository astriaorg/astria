use astria_core::{
    generated::astria::execution::v2::{
        CommitmentState,
        ExecutionSessionParameters,
    },
    primitive::v1::RollupId,
    Protobuf as _,
};

use crate::executor::State;

pub(crate) fn make_commitment_state() -> CommitmentState {
    let firm = astria_core::generated::astria::execution::v2::ExecutedBlockMetadata {
        number: 1,
        hash: hex::encode([42u8; 32]).to_string(),
        parent_hash: hex::encode([41u8; 32]).to_string(),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 123_456,
            nanos: 789,
        }),
    };
    let soft = astria_core::generated::astria::execution::v2::ExecutedBlockMetadata {
        number: 2,
        hash: hex::encode([43u8; 32]).to_string(),
        parent_hash: hex::encode([42u8; 32]).to_string(),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 123_456,
            nanos: 789,
        }),
    };

    CommitmentState {
        soft_executed_block_metadata: Some(soft),
        firm_executed_block_metadata: Some(firm),
        lowest_celestia_search_height: 1,
    }
}

pub(crate) fn make_execution_session_parameters() -> ExecutionSessionParameters {
    let rollup_id = RollupId::new([24; 32]);
    ExecutionSessionParameters {
        rollup_id: Some(rollup_id.into_raw()),
        rollup_start_block_number: 1,
        rollup_end_block_number: 10,
        sequencer_chain_id: "test-sequencer-0".to_string(),
        sequencer_start_block_height: 10,
        celestia_chain_id: "test-celestia-0".to_string(),
        celestia_search_height_max_look_ahead: 90,
    }
}

pub(crate) fn make_rollup_state(
    execution_session_id: String,
    execution_session_parameters: ExecutionSessionParameters,
    commitment_state: CommitmentState,
) -> State {
    let execution_session_parameters =
        astria_core::execution::v2::ExecutionSessionParameters::try_from_raw(
            execution_session_parameters,
        )
        .unwrap();
    let commitment_state =
        astria_core::execution::v2::CommitmentState::try_from_raw(commitment_state).unwrap();
    State::try_from_execution_session_parameters_and_commitment_state(
        execution_session_id,
        execution_session_parameters,
        commitment_state,
        crate::config::CommitLevel::SoftAndFirm,
    )
    .unwrap()
}
