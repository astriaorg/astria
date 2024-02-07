use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    sequencer::v1alpha1::RollupId,
};
use celestia_client::celestia_types::Height as CelestiaHeight;
use sequencer_client::tendermint::block::Height;

#[derive(Debug)]
pub(crate) struct State {
    inner: Option<StateImpl>,
}

#[derive(Debug)]
struct StateImpl {
    genesis_info: GenesisInfo,
    commitment_state: CommitmentState,

    next_firm_sequencer_height: Height,
    next_soft_sequencer_height: Height,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            inner: None,
        }
    }

    pub(super) fn init(&mut self, genesis_info: GenesisInfo, commitment_state: CommitmentState) {
        self.inner
            .replace(StateImpl::new(genesis_info, commitment_state));
    }

    pub(crate) fn is_init(&self) -> bool {
        self.inner.is_some()
    }

    /// Updates the tracked state if `commitment_state` is different.
    pub(super) fn update_if_modified(&mut self, commitment_state: CommitmentState) -> bool {
        self.inner
            .as_mut()
            .expect("the state is initialized")
            .update_if_modified(commitment_state)
    }
}

macro_rules! forward_impls {
    ($([$fn:ident -> $ret:ty]),*$(,)?) => {
        impl State {
            $(
            pub(crate) fn $fn(&self) -> $ret {
                self.inner
                    .as_ref()
                    .expect("the state is initialized")
                    .$fn()
            }
            )*
        }
    }
}

forward_impls!(
    [firm -> &Block],
    [soft -> &Block],
    [firm_parent_hash -> [u8; 32]],
    [soft_parent_hash -> [u8; 32]],
    [celestia_base_block_height -> CelestiaHeight],
    [rollup_id -> RollupId],
    [next_firm_sequencer_height -> Height],
    [next_soft_sequencer_height -> Height],
);

impl StateImpl {
    pub(super) fn new(genesis_info: GenesisInfo, commitment_state: CommitmentState) -> Self {
        let next_firm_sequencer_height = map_rollup_height_to_sequencer_height(
            genesis_info.sequencer_genesis_block_height(),
            commitment_state.firm().number(),
        );

        let next_soft_sequencer_height = map_rollup_height_to_sequencer_height(
            genesis_info.sequencer_genesis_block_height(),
            commitment_state.soft().number(),
        );
        Self {
            genesis_info,
            commitment_state,
            next_firm_sequencer_height,
            next_soft_sequencer_height,
        }
    }

    /// Updates the tracked state if `commitment_state` is different.
    pub(super) fn update_if_modified(&mut self, commitment_state: CommitmentState) -> bool {
        let changed = self.commitment_state != commitment_state;
        if changed {
            self.next_firm_sequencer_height = map_rollup_height_to_sequencer_height(
                self.genesis_info.sequencer_genesis_block_height(),
                commitment_state.firm().number(),
            );
            self.next_soft_sequencer_height = map_rollup_height_to_sequencer_height(
                self.genesis_info.sequencer_genesis_block_height(),
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

    pub(super) fn celestia_base_block_height(&self) -> CelestiaHeight {
        self.genesis_info.celestia_base_block_height()
    }

    pub(super) fn rollup_id(&self) -> RollupId {
        self.genesis_info.rollup_id()
    }

    pub(super) fn next_firm_sequencer_height(&self) -> Height {
        self.next_firm_sequencer_height
    }

    pub(crate) fn next_soft_sequencer_height(&self) -> Height {
        self.next_soft_sequencer_height
    }
}

// Maps a rollup height to a sequencer heights.
/// # Panics
///
/// Panics if adding the integers overflows. Comet BFT has hopefully migrated
/// to `uint64` heights by the times this becomes an issue.
fn map_rollup_height_to_sequencer_height(
    first_sequencer_height: Height,
    current_rollup_height: u32,
) -> Height {
    let first_sequencer_height: u32 = first_sequencer_height
        .value()
        .try_into()
        .expect("should not fail; cometbft heights are internally represented as int64");
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

#[cfg(test)]
mod tests {
    use astria_core::{
        generated::execution::v1alpha2 as raw,
        Protobuf as _,
    };
    use prost_types::Timestamp;

    use super::*;

    fn make_commitment_state() -> CommitmentState {
        let firm = Block::try_from_raw(raw::Block {
            number: 1,
            hash: vec![42u8; 32],
            parent_block_hash: vec![41u8; 32],
            timestamp: Some(Timestamp {
                seconds: 123_456,
                nanos: 789,
            }),
        })
        .unwrap();
        let soft = Block::try_from_raw(raw::Block {
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

    fn make_genesis_info() -> GenesisInfo {
        GenesisInfo::try_from_raw(raw::GenesisInfo {
            rollup_id: vec![24; 32],
            sequencer_genesis_block_height: 10,
            celestia_base_block_height: 1,
            celestia_block_variance: 0,
        })
        .unwrap()
    }

    fn make_state() -> State {
        let mut state = State::new();
        state.init(make_genesis_info(), make_commitment_state());
        state
    }

    #[test]
    fn next_firm_sequencer_height_is_correct() {
        let state = make_state();
        assert_eq!(Height::from(11u32), state.next_firm_sequencer_height(),);
    }

    #[test]
    fn next_soft_sequencer_height_is_correct() {
        let state = make_state();
        assert_eq!(Height::from(12u32), state.next_soft_sequencer_height(),);
    }

    #[track_caller]
    fn assert_height_is_correct(left: u32, right: u32, expected: u32) {
        assert_eq!(
            Height::from(expected),
            map_rollup_height_to_sequencer_height(Height::from(left), right),
        );
    }

    #[test]
    fn mapping_rollup_height_to_sequencer_height_works() {
        assert_height_is_correct(0, 0, 0);
        assert_height_is_correct(0, 1, 1);
        assert_height_is_correct(1, 0, 1);
    }
}
