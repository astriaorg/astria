use astria_core::execution::v1alpha2::{
    Block,
    CommitmentState,
};
use sequencer_client::tendermint::block::Height;

// Maps a rollup height to a sequencer heights.
/// # Panics
///
/// Panics if adding the integers overflows. Comet BFT has hopefully migrated
/// to `uint64` heights by the times this becomes an issue.
fn map_rollup_height_to_sequencer_height(
    first_sequencer_height: u32,
    current_rollup_height: u32,
) -> Height {
    first_sequencer_height
        .checked_add(current_rollup_height)
        .expect(
            "should not overflow; if this overflows either the first sequencer height or current \
             rollup height have been incorrectly set incorrectly set, are in the future, or the \
             rollup/sequencer have been running for several decades without cometbft ever \
             receiving an update to uin64 heights",
        )
        .into()
}

#[derive(Debug)]
pub(super) struct TrackState {
    // The sequencer height that contains the first block of the executed-upon rollup.
    sequencer_height_with_first_rollup_block: u32,

    state: CommitmentState,
}

impl TrackState {
    pub(super) fn with_state(state: CommitmentState) -> Self {
        Self {
            sequencer_height_with_first_rollup_block: 0,
            state,
        }
    }

    pub(super) fn set_state(&mut self, state: CommitmentState) {
        self.state = state;
    }

    pub(super) fn set_sequencer_height_with_first_rollup_block(
        &mut self,
        sequencer_height_with_first_rollup_block: u32,
    ) {
        self.sequencer_height_with_first_rollup_block = sequencer_height_with_first_rollup_block;
    }

    pub(super) fn firm(&self) -> &Block {
        self.state.firm()
    }

    pub(super) fn soft(&self) -> &Block {
        self.state.soft()
    }

    pub(super) fn firm_parent_hash(&self) -> [u8; 32] {
        self.firm().hash()
    }

    pub(super) fn soft_parent_hash(&self) -> [u8; 32] {
        self.soft().hash()
    }

    pub(super) fn next_firm_sequencer_height(&self) -> Height {
        map_rollup_height_to_sequencer_height(
            self.sequencer_height_with_first_rollup_block,
            self.state.firm().number(),
        )
    }

    pub(super) fn next_soft_sequencer_height(&self) -> Height {
        map_rollup_height_to_sequencer_height(
            self.sequencer_height_with_first_rollup_block,
            self.state.soft().number(),
        )
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        generated::execution::v1alpha2::Block as RawBlock,
        Protobuf as _,
    };
    use prost_types::Timestamp;

    use super::*;

    fn make_commitment_state() -> CommitmentState {
        let firm = Block::try_from_raw(RawBlock {
            number: 1,
            hash: vec![42u8; 32],
            parent_block_hash: vec![41u8; 32],
            timestamp: Some(Timestamp {
                seconds: 123_456,
                nanos: 789,
            }),
        })
        .unwrap();
        let soft = Block::try_from_raw(RawBlock {
            number: 2,
            hash: vec![43u8; 32],
            parent_block_hash: vec![42u8; 32],
            timestamp: Some(Timestamp {
                seconds: 123_456,
                nanos: 789,
            }),
        })
        .unwrap();
        CommitmentState::builder()
            .firm(firm)
            .soft(soft)
            .build()
            .unwrap()
    }

    #[test]
    fn next_firm_sequencer_height_is_correct() {
        let commitment_state = make_commitment_state();
        let mut tracker = TrackState::with_state(commitment_state);
        tracker.set_sequencer_height_with_first_rollup_block(10);
        assert_eq!(Height::from(11u32), tracker.next_firm_sequencer_height(),);
    }

    #[test]
    fn next_soft_sequencer_height_is_correct() {
        let commitment_state = make_commitment_state();
        let mut tracker = TrackState::with_state(commitment_state);
        tracker.set_sequencer_height_with_first_rollup_block(10);
        assert_eq!(Height::from(12u32), tracker.next_soft_sequencer_height(),);
    }

    #[track_caller]
    fn assert_height_is_correct(left: u32, right: u32, expected: u32) {
        assert_eq!(
            Height::from(expected),
            map_rollup_height_to_sequencer_height(left, right),
        );
    }

    #[test]
    fn mapping_rollup_height_to_sequencer_height_works() {
        assert_height_is_correct(0, 0, 0);
        assert_height_is_correct(0, 1, 1);
        assert_height_is_correct(1, 0, 1);
    }

    #[test]
    #[should_panic]
    fn too_large_heights_panic() {
        map_rollup_height_to_sequencer_height(2u32.pow(31), 2u32.pow(31));
    }
}
