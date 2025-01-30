use astria_core::{
    generated::astria::execution::v1::{
        CommitmentState,
        GenesisInfo,
    },
    primitive::v1::RollupId,
    Protobuf as _,
};

use crate::executor::State;

pub(crate) fn make_commitment_state() -> CommitmentState {
    let firm = astria_core::generated::astria::execution::v1::Block {
        number: 1,
        hash: vec![42u8; 32].into(),
        parent_block_hash: vec![41u8; 32].into(),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 123_456,
            nanos: 789,
        }),
    };
    let soft = astria_core::generated::astria::execution::v1::Block {
        number: 2,
        hash: vec![43u8; 32].into(),
        parent_block_hash: vec![42u8; 32].into(),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 123_456,
            nanos: 789,
        }),
    };

    CommitmentState {
        soft: Some(soft),
        firm: Some(firm),
        base_celestia_height: 1,
    }
}

pub(crate) fn make_genesis_info() -> GenesisInfo {
    let rollup_id = RollupId::new([24; 32]);
    GenesisInfo {
        rollup_id: Some(rollup_id.to_raw()),
        sequencer_start_height: 10,
        celestia_block_variance: 0,
        rollup_start_block_number: 0,
        rollup_stop_block_number: 90,
        sequencer_chain_id: "test-sequencer-0".to_string(),
        celestia_chain_id: "test-celestia-0".to_string(),
        halt_at_stop_height: false,
    }
}

pub(crate) fn make_rollup_state(
    genesis_info: GenesisInfo,
    commitment_state: CommitmentState,
) -> State {
    let genesis_info = astria_core::execution::v1::GenesisInfo::try_from_raw(genesis_info).unwrap();
    let commitment_state =
        astria_core::execution::v1::CommitmentState::try_from_raw(commitment_state).unwrap();
    State::try_from_genesis_info_and_commitment_state(genesis_info, commitment_state).unwrap()
}

pub(crate) fn test_rollup_state() -> State {
    make_rollup_state(make_genesis_info(), make_commitment_state())
}
