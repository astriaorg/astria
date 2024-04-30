use astria_core::{
    self,
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    generated::execution::v1alpha2 as raw,
    Protobuf as _,
};
use bytes::Bytes;

use super::{
    should_execute_firm_block,
    state::{
        StateReceiver,
        StateSender,
    },
    RollupId,
};
use crate::config::CommitLevel;

const ROLLUP_ID: RollupId = RollupId::new([42u8; 32]);

fn make_block(number: u32) -> raw::Block {
    raw::Block {
        number,
        hash: Bytes::from_static(&[0u8; 32]),
        parent_block_hash: Bytes::from_static(&[0u8; 32]),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 0,
            nanos: 0,
        }),
    }
}

struct MakeState {
    firm: u32,
    soft: u32,
}

fn make_state(
    MakeState {
        firm,
        soft,
    }: MakeState,
) -> (StateSender, StateReceiver) {
    let genesis_info = GenesisInfo::try_from_raw(raw::GenesisInfo {
        rollup_id: Bytes::copy_from_slice(ROLLUP_ID.as_ref()),
        sequencer_genesis_block_height: 1,
        celestia_base_block_height: 1,
        celestia_block_variance: 1,
    })
    .unwrap();
    let commitment_state = CommitmentState::try_from_raw(raw::CommitmentState {
        firm: Some(make_block(firm)),
        soft: Some(make_block(soft)),
    })
    .unwrap();
    let (mut tx, rx) = super::state::channel();
    tx.try_init(genesis_info, commitment_state).unwrap();
    (tx, rx)
}

#[track_caller]
fn assert_contract_fulfilled(kind: super::ExecutionKind, state: MakeState, number: u32) {
    let block = Block::try_from_raw(make_block(number)).unwrap();
    let mut state = make_state(state);
    super::does_block_response_fulfill_contract(&mut state.0, kind, &block)
        .expect("number stored in response block must be one more than in tracked state");
}

#[track_caller]
fn assert_contract_violated(kind: super::ExecutionKind, state: MakeState, number: u32) {
    let block = Block::try_from_raw(make_block(number)).unwrap();
    let mut state = make_state(state);
    super::does_block_response_fulfill_contract(&mut state.0, kind, &block).expect_err(
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
