//! After being created the state must be primed with [`State::init`] before any of
//! the other methods can be used. Otherwise, they will panic.
//!
//! The inner state must not be unset after having been set.
use std::num::NonZeroU64;

use astria_core::{
    execution::v2::{
        CommitmentState,
        ExecutedBlockMetadata,
        ExecutionSession,
        ExecutionSessionParameters,
    },
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    eyre,
};
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
    "could not map rollup number to sequencer height for {map_purpose}: the operation \
     `{sequencer_start_height} + ({rollup_number} - {rollup_start_block_number})` failed because \
     `{issue}`"
)]
pub(crate) struct InvalidState {
    map_purpose: &'static str,
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
            .expect("the tracked state must never be set to an invalid state. this is a bug")
    }

    pub(crate) fn next_expected_soft_sequencer_height(&self) -> SequencerHeight {
        self.inner
            .borrow()
            .next_expected_soft_sequencer_height()
            .expect("the tracked state must never be set to an invalid state. this is a bug")
    }

    #[instrument(skip_all)]
    pub(crate) async fn next_expected_soft_height_if_changed(
        &mut self,
    ) -> Result<SequencerHeight, RecvError> {
        self.inner.changed().await?;
        Ok(self.next_expected_soft_sequencer_height())
    }

    pub(crate) fn sequencer_stop_height(&self) -> Option<NonZeroU64> {
        let rollup_end_block_number = self.inner.borrow().rollup_end_block_number()?;
        let sequencer_start_height = self.inner.borrow().sequencer_start_block_height();
        let rollup_start_block_number = self.inner.borrow().rollup_start_block_number();
        NonZeroU64::new(
            map_rollup_number_to_sequencer_height(
                sequencer_start_height,
                rollup_start_block_number,
                rollup_end_block_number.get(),
            )
            .expect("the tracked state must never be set to an invalid state. this is a bug")
            .into(),
        )
    }
}

pub(super) struct StateSender {
    inner: watch::Sender<State>,
}

impl std::fmt::Display for StateSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(&*self.inner.borrow()).map_err(|_| std::fmt::Error)?;
        f.write_str(&s)
    }
}

fn map_firm_to_sequencer_height(
    execution_session_parameters: &ExecutionSessionParameters,
    commitment_state: &CommitmentState,
) -> Result<SequencerHeight, InvalidState> {
    let sequencer_start_height = execution_session_parameters.sequencer_start_block_height();
    let rollup_start_block_number = execution_session_parameters.rollup_start_block_number();
    let rollup_number = commitment_state.firm().number();

    map_rollup_number_to_sequencer_height(
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    )
    .map_err(|issue| InvalidState {
        map_purpose: "firm commitment",
        issue,
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    })
}

fn map_soft_to_sequencer_height(
    execution_session_parameters: &ExecutionSessionParameters,
    commitment_state: &CommitmentState,
) -> Result<SequencerHeight, InvalidState> {
    let sequencer_start_height = execution_session_parameters.sequencer_start_block_height();
    let rollup_start_block_number = execution_session_parameters.rollup_start_block_number();
    let rollup_number = commitment_state.soft().number();

    map_rollup_number_to_sequencer_height(
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    )
    .map_err(|issue| InvalidState {
        map_purpose: "soft commitment",
        issue,
        sequencer_start_height,
        rollup_start_block_number,
        rollup_number,
    })
}

impl StateSender {
    pub(super) fn subscribe(&self) -> StateReceiver {
        StateReceiver {
            inner: self.inner.subscribe(),
        }
    }

    pub(super) fn try_update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
        commit_level: crate::config::CommitLevel,
    ) -> Result<(), InvalidState> {
        let execution_session_parameters = self.execution_session_parameters();
        if commit_level.is_with_firm() {
            let _ = map_firm_to_sequencer_height(&execution_session_parameters, &commitment_state)?;
        }
        if commit_level.is_with_soft() {
            let _ = map_soft_to_sequencer_height(&execution_session_parameters, &commitment_state)?;
        }
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
    [execution_session_parameters -> ExecutionSessionParameters],
    [execution_session_id -> String],
    [firm -> ExecutedBlockMetadata],
    [soft -> ExecutedBlockMetadata],
    [firm_number -> u64],
    [soft_number -> u64],
    [firm_hash -> String],
    [soft_hash -> String],
    [rollup_id -> RollupId],
    [sequencer_start_block_height -> u64],
    [lowest_celestia_search_height -> u64],
    [celestia_search_height_max_look_ahead -> u64],
    [rollup_start_block_number -> u64],
    [rollup_end_block_number -> Option<NonZeroU64>],
    [has_firm_number_reached_stop_height -> bool],
    [has_soft_number_reached_stop_height -> bool],
);

forward_impls!(
    StateReceiver:
    [lowest_celestia_search_height -> u64],
    [celestia_search_height_max_look_ahead -> u64],
    [rollup_id -> RollupId],
    [sequencer_chain_id -> String],
    [celestia_chain_id -> String],
);

/// `State` tracks the genesis info and commitment state of the remote rollup node.
#[derive(Clone, Debug, serde::Serialize)]
#[expect(
    clippy::struct_field_names,
    reason = "`commitment_state` is the most accurate name"
)]
pub(crate) struct State {
    execution_session_id: String,
    execution_session_parameters: ExecutionSessionParameters,
    commitment_state: CommitmentState,
}

impl State {
    pub(crate) fn try_from_execution_session(
        execution_session: &ExecutionSession,
        commit_level: crate::config::CommitLevel,
    ) -> Result<Self, InvalidState> {
        let execution_session_parameters = execution_session.execution_session_parameters();
        let commitment_state = execution_session.commitment_state();
        if commit_level.is_with_firm() {
            let _ = map_firm_to_sequencer_height(execution_session_parameters, commitment_state)?;
        }
        if commit_level.is_with_soft() {
            let _ = map_soft_to_sequencer_height(execution_session_parameters, commitment_state)?;
        }
        let execution_session_parameters = execution_session.execution_session_parameters();
        if let Some(rollup_end_block_number) =
            execution_session_parameters.rollup_end_block_number()
        {
            let _ = map_rollup_number_to_sequencer_height(
                execution_session_parameters.sequencer_start_block_height(),
                execution_session_parameters.rollup_start_block_number(),
                rollup_end_block_number.get(),
            )
            .map_err(|issue| InvalidState {
                map_purpose: "rollup end block number",
                issue,
                sequencer_start_height: execution_session_parameters.sequencer_start_block_height(),
                rollup_start_block_number: execution_session_parameters.rollup_start_block_number(),
                rollup_number: rollup_end_block_number.get(),
            })?;
        };
        Ok(State {
            execution_session_id: execution_session.session_id().to_string(),
            execution_session_parameters: execution_session_parameters.clone(),
            commitment_state: commitment_state.clone(),
        })
    }

    /// Returns if the tracked firm state of the rollup has reached the rollup stop block number.
    pub(crate) fn has_firm_number_reached_stop_height(&self) -> bool {
        let Some(rollup_end_block_number) = self.rollup_end_block_number() else {
            return false;
        };
        self.commitment_state.firm().number() >= rollup_end_block_number.get()
    }

    /// Returns if the tracked soft state of the rollup has reached the rollup stop block number.
    pub(crate) fn has_soft_number_reached_stop_height(&self) -> bool {
        let Some(rollup_end_block_number) = self.rollup_end_block_number() else {
            return false;
        };
        self.commitment_state.soft().number() >= rollup_end_block_number.get()
    }

    /// Sets the inner commitment state.
    fn set_commitment_state(&mut self, commitment_state: CommitmentState) {
        self.commitment_state = commitment_state;
    }

    fn execution_session_parameters(&self) -> &ExecutionSessionParameters {
        &self.execution_session_parameters
    }

    fn execution_session_id(&self) -> String {
        self.execution_session_id.clone()
    }

    fn firm(&self) -> &ExecutedBlockMetadata {
        self.commitment_state.firm()
    }

    fn soft(&self) -> &ExecutedBlockMetadata {
        self.commitment_state.soft()
    }

    pub(crate) fn firm_number(&self) -> u64 {
        self.commitment_state.firm().number()
    }

    pub(crate) fn soft_number(&self) -> u64 {
        self.commitment_state.soft().number()
    }

    fn firm_hash(&self) -> String {
        self.firm().hash().to_string()
    }

    fn soft_hash(&self) -> String {
        self.soft().hash().to_string()
    }

    fn lowest_celestia_search_height(&self) -> u64 {
        self.commitment_state.lowest_celestia_search_height()
    }

    fn celestia_search_height_max_look_ahead(&self) -> u64 {
        self.execution_session_parameters
            .celestia_search_height_max_look_ahead()
    }

    pub(crate) fn sequencer_start_block_height(&self) -> u64 {
        self.execution_session_parameters
            .sequencer_start_block_height()
    }

    fn sequencer_chain_id(&self) -> String {
        self.execution_session_parameters
            .sequencer_chain_id()
            .to_string()
    }

    fn celestia_chain_id(&self) -> String {
        self.execution_session_parameters
            .celestia_chain_id()
            .to_string()
    }

    fn rollup_id(&self) -> RollupId {
        self.execution_session_parameters.rollup_id()
    }

    pub(crate) fn rollup_start_block_number(&self) -> u64 {
        self.execution_session_parameters
            .rollup_start_block_number()
    }

    pub(crate) fn rollup_end_block_number(&self) -> Option<NonZeroU64> {
        self.execution_session_parameters.rollup_end_block_number()
    }

    pub(crate) fn firm_block_number_as_sequencer_height(&self) -> SequencerHeight {
        map_firm_to_sequencer_height(&self.execution_session_parameters, &self.commitment_state)
            .expect(
                "state must only contain numbers that can be mapped to sequencer heights; this is \
                 enforced by its constructor and/or setter",
            )
    }

    pub(crate) fn soft_block_number_as_sequencer_height(&self) -> SequencerHeight {
        map_soft_to_sequencer_height(&self.execution_session_parameters, &self.commitment_state)
            .expect(
                "state must only contain numbers that can be mapped to sequencer heights; this is \
                 enforced by its constructor and/or setter",
            )
    }

    fn next_expected_firm_sequencer_height(&self) -> Result<SequencerHeight, InvalidState> {
        map_firm_to_sequencer_height(&self.execution_session_parameters, &self.commitment_state)
            .map(SequencerHeight::increment)
    }

    fn next_expected_soft_sequencer_height(&self) -> Result<SequencerHeight, InvalidState> {
        map_soft_to_sequencer_height(&self.execution_session_parameters, &self.commitment_state)
            .map(SequencerHeight::increment)
    }
}

/// Maps a rollup height to a sequencer height.
///
/// Returns error if `sequencer_start_height + (rollup_number - rollup_start_block_number)`
/// is out of range of `u64` or if `rollup_start_block_number` is more than 1 greater than
/// `rollup_number`.
fn map_rollup_number_to_sequencer_height(
    sequencer_start_height: u64,
    rollup_start_block_number: u64,
    rollup_number: u64,
) -> Result<SequencerHeight, &'static str> {
    if rollup_start_block_number > (rollup_number.checked_add(1).ok_or("overflows u64::MAX")?) {
        return Err("rollup start height exceeds rollup number + 1");
    }
    let sequencer_height = sequencer_start_height
        .checked_add(rollup_number)
        .ok_or("overflows u64::MAX")?
        .checked_sub(rollup_start_block_number)
        .ok_or("(sequencer height + rollup number - rollup start height) is negative")?;
    sequencer_height
        .try_into()
        .map_err(|_| "overflows u64::MAX")
}

/// Maps a sequencer height to a rollup height.
///
/// Returns `None` if `sequencer_height - sequencer_start_height + rollup_start_block_number`
/// underflows or if the result does not fit in `u64`.
pub(super) fn try_map_sequencer_height_to_rollup_height(
    sequencer_start_height: u64,
    rollup_start_block_number: u64,
    sequencer_height: SequencerHeight,
) -> eyre::Result<u64> {
    sequencer_height
        .value()
        .checked_sub(sequencer_start_height)
        .ok_or_else(|| {
            eyre!(format!(
                "operation `sequencer_height {{{sequencer_height}}} - sequencer_start_height \
                 {{{sequencer_start_height}}}` underflowed"
            ))
        })?
        .checked_add(rollup_start_block_number)
        .ok_or_else(|| {
            eyre!(format!(
                "operation `(sequencer_height {{{sequencer_height}}} - sequencer_start_height \
                 {{{sequencer_start_height}}}) + rollup_start_block_number \
                 {{{rollup_start_block_number}}}` overflowed"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        make_commitment_state,
        make_execution_session_parameters,
        make_rollup_state,
    };

    fn make_channel() -> (StateSender, StateReceiver) {
        super::channel(make_rollup_state(
            "test_session".to_string(),
            make_execution_session_parameters(),
            make_commitment_state(),
        ))
    }

    #[test]
    fn next_firm_sequencer_height_is_correct() {
        let (_, rx) = make_channel();
        assert_eq!(
            SequencerHeight::from(11u32),
            rx.next_expected_firm_sequencer_height(),
        );
    }

    #[test]
    fn next_soft_sequencer_height_is_correct() {
        let (_, rx) = make_channel();
        assert_eq!(
            SequencerHeight::from(12u32),
            rx.next_expected_soft_sequencer_height(),
        );
    }

    #[track_caller]
    fn assert_height_is_correct(
        sequencer_start_height: u64,
        rollup_start_number: u64,
        rollup_number: u64,
        expected_sequencer_height: u32,
    ) {
        assert_eq!(
            SequencerHeight::from(expected_sequencer_height),
            map_rollup_number_to_sequencer_height(
                sequencer_start_height,
                rollup_start_number,
                rollup_number,
            )
            .unwrap()
        );
    }

    #[should_panic = "rollup start height exceeds rollup number"]
    #[test]
    fn is_error_if_rollup_start_exceeds_current_number_plus_one() {
        map_rollup_number_to_sequencer_height(10, 11, 9).unwrap();
    }

    #[test]
    fn mapping_rollup_height_to_sequencer_height_works() {
        assert_height_is_correct(0, 0, 0, 0);
        assert_height_is_correct(0, 1, 1, 0);
        assert_height_is_correct(1, 0, 1, 2);
    }
}
