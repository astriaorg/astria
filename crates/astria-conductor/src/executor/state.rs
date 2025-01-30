//! After being created the state must be primed with [`State::init`] before any of
//! the other methods can be used. Otherwise, they will panic.
//!
//! The inner state must not be unset after having been set.
use std::num::NonZeroU64;

use astria_core::{
    execution::v1::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use bytes::Bytes;
use sequencer_client::tendermint::block::Height as SequencerHeight;
use tokio::sync::watch::{
    self,
    error::RecvError,
};
use tracing::instrument;

pub(super) fn channel(state: State) -> (StateSender, StateReceiver) {
    let (tx, rx) = watch::channel(state);
    let sender = StateSender {
        inner: tx,
    };
    let receiver = StateReceiver {
        inner: rx,
    };
    (sender, receiver)
}

#[derive(Debug, thiserror::Error)]
#[error(
    "could not map rollup number to sequencer height for commitment type `{commitment_type}`: the \
     operation `{sequencer_start_height} + ({rollup_number} - {rollup_start_block_number}`) \
     failed because `{issue}`"
)]
pub(crate) struct InvalidState {
    commitment_type: &'static str,
    issue: &'static str,
    sequencer_start_height: u64,
    rollup_start_block_number: u64,
    rollup_number: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct StateReceiver {
    inner: watch::Receiver<State>,
}

impl StateReceiver {
    pub(crate) fn next_expected_firm_sequencer_height(&self) -> SequencerHeight {
        self.inner
            .borrow()
            .next_expected_firm_sequencer_height()
            .expect(
                "the tracked state must never be set to a genesis/commitment state that cannot be \
                 mapped to a cometbft Sequencer height",
            )
    }

    pub(crate) fn next_expected_soft_sequencer_height(&self) -> SequencerHeight {
        self.inner
            .borrow()
            .next_expected_soft_sequencer_height()
            .expect(
                "the tracked state must never be set to a genesis/commitment state that cannot be \
                 mapped to a cometbft Sequencer height",
            )
    }

    #[instrument(skip_all)]
    pub(crate) async fn next_expected_soft_height_if_changed(
        &mut self,
    ) -> Result<SequencerHeight, RecvError> {
        self.inner.changed().await?;
        Ok(self.next_expected_soft_sequencer_height())
    }

    pub(crate) fn sequencer_stop_height(&self) -> eyre::Result<Option<NonZeroU64>> {
        let Some(rollup_stop_block_number) = self.inner.borrow().rollup_stop_block_number() else {
            return Ok(None);
        };
        let sequencer_start_height = self.inner.borrow().sequencer_start_height();
        let rollup_start_block_number = self.inner.borrow().rollup_start_block_number();
        Ok(NonZeroU64::new(
            map_rollup_number_to_sequencer_height(
                sequencer_start_height,
                rollup_start_block_number,
                rollup_stop_block_number
                    .get()
                    .try_into()
                    .wrap_err("rollup stop block number overflows u32::MAX")?,
            )
            .map_err(|e| {
                eyre!(e).wrap_err("failed to map rollup stop block number to sequencer height")
            })?
            .into(),
        ))
    }
}

pub(super) struct StateSender {
    inner: watch::Sender<State>,
}

impl std::fmt::Display for StateSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(&*self.inner.borrow()).unwrap();
        f.write_str(&s)
    }
}

fn map_firm_to_sequencer_height(
    genesis_info: &GenesisInfo,
    commitment_state: &CommitmentState,
) -> Result<SequencerHeight, InvalidState> {
    let sequencer_start_height = genesis_info.sequencer_start_height();
    let rollup_start_block_number = genesis_info.rollup_start_block_number();
    let rollup_number = commitment_state.firm().number();

    map_rollup_number_to_sequencer_height(
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    )
    .map_err(|issue| InvalidState {
        commitment_type: "firm",
        issue,
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number: rollup_number.into(),
    })
}

fn map_soft_to_sequencer_height(
    genesis_info: &GenesisInfo,
    commitment_state: &CommitmentState,
) -> Result<SequencerHeight, InvalidState> {
    let sequencer_start_height = genesis_info.sequencer_start_height();
    let rollup_start_block_number = genesis_info.rollup_start_block_number();
    let rollup_number = commitment_state.soft().number();

    map_rollup_number_to_sequencer_height(
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    )
    .map_err(|issue| InvalidState {
        commitment_type: "soft",
        issue,
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number: rollup_number.into(),
    })
}

impl StateSender {
    pub(super) fn subscribe(&self) -> StateReceiver {
        StateReceiver {
            inner: self.inner.subscribe(),
        }
    }

    /// Calculates the maximum allowed spread between firm and soft commitments heights.
    ///
    /// The maximum allowed spread is taken as `max_spread = variance * 6`, where `variance`
    /// is the `celestia_block_variance` as defined in the rollup node's genesis that this
    /// executor/conductor talks to.
    ///
    /// The heuristic 6 is the largest number of Sequencer heights that will be found at
    /// one Celestia height.
    ///
    /// # Panics
    /// Panics if the `u32` underlying the celestia block variance tracked in the state could
    /// not be converted to a `usize`. This should never happen on any reasonable architecture
    /// that Conductor will run on.
    pub(super) fn calculate_max_spread(&self) -> usize {
        usize::try_from(self.celestia_block_variance())
            .expect("converting a u32 to usize should work on any architecture conductor runs on")
            .saturating_mul(6)
    }

    pub(super) fn try_update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
    ) -> Result<(), InvalidState> {
        let genesis_info = self.genesis_info();
        let _ = map_firm_to_sequencer_height(&genesis_info, &commitment_state)?;
        let _ = map_soft_to_sequencer_height(&genesis_info, &commitment_state)?;
        self.inner.send_modify(move |state| {
            state.set_commitment_state(commitment_state);
        });
        Ok(())
    }

    pub(super) fn get(&self) -> tokio::sync::watch::Ref<'_, State> {
        self.inner.borrow()
    }

    pub(super) fn next_expected_firm_sequencer_height(&self) -> SequencerHeight {
        self.inner
            .borrow()
            .next_expected_firm_sequencer_height()
            .expect(
                "the tracked state must never be set to a genesis/commitment state that cannot be \
                 mapped to a cometbft Sequencer height",
            )
    }

    pub(super) fn next_expected_soft_sequencer_height(&self) -> SequencerHeight {
        self.inner
            .borrow()
            .next_expected_soft_sequencer_height()
            .expect(
                "the tracked state must never be set to a genesis/commitment state that cannot be \
                 mapped to a cometbft Sequencer height",
            )
    }
}

macro_rules! forward_impls {
    ($target:ident: $([$fn:ident -> $ret:ty]),*$(,)?) => {
        impl $target {
            $(
            pub(crate) fn $fn(&self) -> $ret {
                self.inner
                    .borrow()
                    .$fn()
                    .clone()
            }
            )*
        }
    }
}

forward_impls!(
    StateSender:
    [genesis_info -> GenesisInfo],
    [firm -> Block],
    [soft -> Block],
    [firm_number -> u32],
    [soft_number -> u32],
    [firm_hash -> Bytes],
    [soft_hash -> Bytes],
    [celestia_block_variance -> u64],
    [rollup_id -> RollupId],
    [sequencer_start_height -> u64],
    [celestia_base_block_height -> u64],
    [rollup_start_block_number -> u64],
    [rollup_stop_block_number -> Option<NonZeroU64>],
    [has_firm_number_reached_stop_height -> bool],
    [has_soft_number_reached_stop_height -> bool],
);

forward_impls!(
    StateReceiver:
    [celestia_base_block_height -> u64],
    [celestia_block_variance -> u64],
    [rollup_id -> RollupId],
    [sequencer_chain_id -> String],
    [celestia_chain_id -> String],
);

/// `State` tracks the genesis info and commitment state of the remote rollup node.
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct State {
    commitment_state: CommitmentState,
    genesis_info: GenesisInfo,
}

impl State {
    pub(crate) fn try_from_genesis_info_and_commitment_state(
        genesis_info: GenesisInfo,
        commitment_state: CommitmentState,
    ) -> Result<Self, InvalidState> {
        let _ = map_firm_to_sequencer_height(&genesis_info, &commitment_state)?;
        let _ = map_soft_to_sequencer_height(&genesis_info, &commitment_state)?;
        Ok(State {
            commitment_state,
            genesis_info,
        })
    }

    /// Returns if the tracked firm state of the rollup has reached the rollup stop block number.
    pub(crate) fn has_firm_number_reached_stop_height(&self) -> bool {
        let Some(rollup_stop_block_number) = self.rollup_stop_block_number() else {
            return false;
        };
        u64::from(self.commitment_state.firm().number()) >= rollup_stop_block_number.get()
    }

    /// Returns if the tracked soft state of the rollup has reached the rollup stop block number.
    pub(crate) fn has_soft_number_reached_stop_height(&self) -> bool {
        let Some(rollup_stop_block_number) = self.rollup_stop_block_number() else {
            return false;
        };
        u64::from(self.commitment_state.soft().number()) >= rollup_stop_block_number.get()
    }

    /// Sets the inner commitment state.
    fn set_commitment_state(&mut self, commitment_state: CommitmentState) {
        self.commitment_state = commitment_state;
    }

    fn genesis_info(&self) -> &GenesisInfo {
        &self.genesis_info
    }

    fn firm(&self) -> &Block {
        self.commitment_state.firm()
    }

    fn soft(&self) -> &Block {
        self.commitment_state.soft()
    }

    pub(crate) fn firm_number(&self) -> u32 {
        self.commitment_state.firm().number()
    }

    pub(crate) fn soft_number(&self) -> u32 {
        self.commitment_state.soft().number()
    }

    fn firm_hash(&self) -> Bytes {
        self.firm().hash().clone()
    }

    fn soft_hash(&self) -> Bytes {
        self.soft().hash().clone()
    }

    fn celestia_base_block_height(&self) -> u64 {
        self.commitment_state.base_celestia_height()
    }

    fn celestia_block_variance(&self) -> u64 {
        self.genesis_info.celestia_block_variance()
    }

    pub(crate) fn sequencer_start_height(&self) -> u64 {
        self.genesis_info.sequencer_start_height()
    }

    pub(crate) fn halt_at_stop_height(&self) -> bool {
        self.genesis_info.halt_at_stop_height()
    }

    fn sequencer_chain_id(&self) -> String {
        self.genesis_info.sequencer_chain_id().to_string()
    }

    fn celestia_chain_id(&self) -> String {
        self.genesis_info.celestia_chain_id().to_string()
    }

    fn rollup_id(&self) -> RollupId {
        self.genesis_info.rollup_id()
    }

    pub(crate) fn rollup_start_block_number(&self) -> u64 {
        self.genesis_info.rollup_start_block_number()
    }

    pub(crate) fn rollup_stop_block_number(&self) -> Option<NonZeroU64> {
        self.genesis_info.rollup_stop_block_number()
    }

    pub(crate) fn firm_block_number_as_sequencer_height(&self) -> SequencerHeight {
        map_firm_to_sequencer_height(&self.genesis_info, &self.commitment_state).expect(
            "state must only contain numbers that can be mapped to sequencer heights; this is \
             enforced by its constructor and/or setter",
        )
    }

    pub(crate) fn soft_block_number_as_sequencer_height(&self) -> SequencerHeight {
        map_soft_to_sequencer_height(&self.genesis_info, &self.commitment_state).expect(
            "state must only contain numbers that can be mapped to sequencer heights; this is \
             enforced by its constructor and/or setter",
        )
    }

    fn next_expected_firm_sequencer_height(&self) -> Result<SequencerHeight, InvalidState> {
        map_firm_to_sequencer_height(&self.genesis_info, &self.commitment_state)
            .map(SequencerHeight::increment)
    }

    fn next_expected_soft_sequencer_height(&self) -> Result<SequencerHeight, InvalidState> {
        map_soft_to_sequencer_height(&self.genesis_info, &self.commitment_state)
            .map(SequencerHeight::increment)
    }
}

/// Maps a rollup height to a sequencer height.
///
/// Returns error if `sequencer_start_height + (rollup_number - rollup_start_block_number)`
/// overflows `u32::MAX`.
fn map_rollup_number_to_sequencer_height(
    sequencer_start_height: u64,
    rollup_start_block_number: u64,
    rollup_number: u32,
) -> Result<SequencerHeight, &'static str> {
    let delta = u64::from(rollup_number)
        .checked_sub(rollup_start_block_number)
        .ok_or("rollup start height exceeds rollup number")?;
    let sequencer_height = sequencer_start_height
        .checked_add(delta)
        .ok_or("overflows u64::MAX")?;
    sequencer_height
        .try_into()
        .map_err(|_| "overflows u32::MAX, the maximum cometbft height")
}

/// Maps a sequencer height to a rollup height.
///
/// Returns `None` if `sequencer_height - sequencer_start_height + rollup_start_block_number`
/// underflows or if the result does not fit in `u32`.
pub(super) fn map_sequencer_height_to_rollup_height(
    sequencer_start_height: u64,
    rollup_start_block_number: u64,
    sequencer_height: SequencerHeight,
) -> Option<u32> {
    sequencer_height
        .value()
        .checked_sub(sequencer_start_height)?
        .checked_add(rollup_start_block_number)?
        .try_into()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_rollup_state;

    fn make_channel() -> (StateSender, StateReceiver) {
        super::channel(test_rollup_state())
    }

    #[test]
    fn next_firm_sequencer_height_is_correct() {
        let (_, rx) = make_channel();
        assert_eq!(
            SequencerHeight::from(12u32),
            rx.next_expected_firm_sequencer_height(),
        );
    }

    #[test]
    fn next_soft_sequencer_height_is_correct() {
        let (_, rx) = make_channel();
        assert_eq!(
            SequencerHeight::from(13u32),
            rx.next_expected_soft_sequencer_height(),
        );
    }

    #[track_caller]
    fn assert_height_is_correct(
        sequencer_start_height: u32,
        rollup_start_number: u32,
        rollup_number: u32,
        expected_sequencer_height: u32,
    ) {
        assert_eq!(
            SequencerHeight::from(expected_sequencer_height),
            map_rollup_number_to_sequencer_height(
                sequencer_start_height.into(),
                rollup_start_number.into(),
                rollup_number,
            )
            .unwrap()
        );
    }

    #[should_panic = "rollup start height exceeds rollup number"]
    #[test]
    fn is_error_if_rollup_start_exceeds_current_number() {
        map_rollup_number_to_sequencer_height(10, 10, 9).unwrap();
    }

    #[test]
    fn mapping_rollup_height_to_sequencer_height_works() {
        assert_height_is_correct(0, 0, 0, 0);
        assert_height_is_correct(0, 1, 1, 0);
        assert_height_is_correct(1, 0, 1, 2);
    }
}
