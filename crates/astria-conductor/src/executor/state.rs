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
pub(crate) struct State {
    commitment_state: CommitmentState,

    next_firm_sequencer_height: Height,
    next_soft_sequencer_height: Height,

    // The sequencer height that contains the first block of the executed-upon rollup.
    sequencer_height_with_first_rollup_block: u32,
}

impl State {
    pub(super) fn new(
        commitment_state: CommitmentState,
        sequencer_height_with_first_rollup_block: u32,
    ) -> Self {
        let next_firm_sequencer_height = map_rollup_height_to_sequencer_height(
            sequencer_height_with_first_rollup_block,
            commitment_state.firm().number(),
        );

        let next_soft_sequencer_height = map_rollup_height_to_sequencer_height(
            sequencer_height_with_first_rollup_block,
            commitment_state.soft().number(),
        );

        Self {
            commitment_state,
            next_firm_sequencer_height,
            next_soft_sequencer_height,
            sequencer_height_with_first_rollup_block,
        }
    }

    /// Updates the tracked state if `commitment_state` is different.
    pub(super) fn update_if_modified(&mut self, commitment_state: CommitmentState) -> bool {
        let changed = self.commitment_state != commitment_state;
        if changed {
            self.next_firm_sequencer_height = map_rollup_height_to_sequencer_height(
                self.sequencer_height_with_first_rollup_block,
                commitment_state.firm().number(),
            );
            self.next_soft_sequencer_height = map_rollup_height_to_sequencer_height(
                self.sequencer_height_with_first_rollup_block,
                commitment_state.soft().number(),
            );
            self.commitment_state = commitment_state;
        }
        changed
    }

    pub(super) fn firm(&self) -> &Block {
        self.commitment_state.firm()
    }

    pub(super) fn soft(&self) -> &Block {
        self.commitment_state.soft()
    }

    pub(super) fn firm_parent_hash(&self) -> [u8; 32] {
        self.firm().hash()
    }

    pub(super) fn soft_parent_hash(&self) -> [u8; 32] {
        self.soft().hash()
    }

    pub(super) fn next_firm_sequencer_height(&self) -> Height {
        self.next_firm_sequencer_height
    }

    pub(crate) fn next_soft_sequencer_height(&self) -> Height {
        self.next_soft_sequencer_height
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
        let state = State::new(commitment_state, 10);
        assert_eq!(Height::from(11u32), state.next_firm_sequencer_height(),);
    }

    #[test]
    fn next_soft_sequencer_height_is_correct() {
        let commitment_state = make_commitment_state();
        let state = State::new(commitment_state, 10);
        assert_eq!(Height::from(12u32), state.next_soft_sequencer_height(),);
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
