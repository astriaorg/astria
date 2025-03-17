use astria_core::{
    self,
    execution::v2::{
        ExecutedBlockMetadata,
        ExecutionSession,
    },
    generated::astria::execution::v2 as raw,
    Protobuf as _,
};

use super::{
    should_execute_firm_block,
    state::{
        State,
        StateReceiver,
        StateSender,
    },
};
use crate::{
    config::CommitLevel,
    test_utils::make_execution_session_parameters,
};

fn make_block_metadata(number: u64) -> raw::ExecutedBlockMetadata {
    raw::ExecutedBlockMetadata {
        number,
        hash: hex::encode([0u8; 32]).to_string(),
        parent_hash: hex::encode([0u8; 32]).to_string(),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 0,
            nanos: 0,
        }),
        sequencer_block_hash: Bytes::new(),
    }
}

struct MakeState {
    firm: u64,
    soft: u64,
}

fn make_state(
    MakeState {
        firm,
        soft,
    }: MakeState,
) -> (StateSender, StateReceiver) {
    let commitment_state = raw::CommitmentState {
        firm_executed_block_metadata: Some(make_block_metadata(firm)),
        soft_executed_block_metadata: Some(make_block_metadata(soft)),
        lowest_celestia_search_height: 1,
    };
    let execution_session = ExecutionSession::try_from_raw(raw::ExecutionSession {
        session_id: "test_execution_session".to_string(),
        execution_session_parameters: Some(make_execution_session_parameters()),
        commitment_state: Some(commitment_state),
    })
    .unwrap();
    let state = State::try_from_execution_session(
        &execution_session,
        crate::config::CommitLevel::SoftAndFirm,
    )
    .unwrap();
    super::state::channel(state)
}

#[track_caller]
fn assert_contract_fulfilled(kind: super::ExecutionKind, state: MakeState, number: u64) {
    let block_metadata = ExecutedBlockMetadata::try_from_raw(make_block_metadata(number)).unwrap();
    let mut state = make_state(state);
    super::does_block_response_fulfill_contract(&mut state.0, kind, &block_metadata)
        .expect("number stored in response block must be one more than in tracked state");
}

#[track_caller]
fn assert_contract_violated(kind: super::ExecutionKind, state: MakeState, number: u64) {
    let block_metadata = ExecutedBlockMetadata::try_from_raw(make_block_metadata(number)).unwrap();
    let mut state = make_state(state);
    super::does_block_response_fulfill_contract(&mut state.0, kind, &block_metadata).expect_err(
        "number stored in response block must not be one more than in tracked
state",
    );
}

#[test]
fn execute_block_contract_violation() {
    use super::ExecutionKind::{
        Firm,
        Soft,
    };
    assert_contract_fulfilled(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        3,
    );

    assert_contract_fulfilled(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        4,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        1,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        2,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        4,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        2,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        3,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        5,
    );
}

#[test]
fn should_execute_firm() {
    use CommitLevel::{
        FirmOnly,
        SoftAndFirm,
        SoftOnly,
    };

    assert!(
        should_execute_firm_block(1, 1, FirmOnly),
        "firm-mode conductors should always execute firm blocks"
    );
    assert!(
        should_execute_firm_block(0, 1, FirmOnly),
        "firm-mode conductors should always execute firm blocks"
    );
    assert!(
        should_execute_firm_block(1, 0, FirmOnly),
        "firm-mode conductors should always execute firm blocks"
    );
    assert!(
        !should_execute_firm_block(1, 1, SoftOnly),
        "soft-mode conductors should never execute firm blocks"
    );
    assert!(
        !should_execute_firm_block(0, 1, SoftOnly),
        "soft-mode conductors should never execute firm blocks"
    );
    assert!(
        !should_execute_firm_block(1, 0, SoftOnly),
        "soft-mode conductors should never execute firm blocks"
    );
    assert!(
        should_execute_firm_block(1, 1, SoftAndFirm),
        "firm-and-soft-mode conductors should execute firm blocks if soft and firm numbers match"
    );
    assert!(
        !should_execute_firm_block(0, 1, SoftAndFirm),
        "firm-and-soft-mode conductors should not execute firm blocks if soft and firm numbers \
         don't match"
    );
    assert!(
        !should_execute_firm_block(1, 0, SoftAndFirm),
        "firm-and-soft-mode conductors should not execute firm blocks if soft and firm numbers \
         don't match"
    );
}
