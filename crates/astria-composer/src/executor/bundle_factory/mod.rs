/// ! This module is responsible for bundling sequence actions into bundles that can be
/// submitted to the sequencer.
use std::{
    collections::VecDeque,
    mem,
};

use astria_core::sequencer::v1::{
    transaction::{
        action::SequenceAction,
        Action,
    },
    ROLLUP_ID_LEN,
};
use tracing::trace;

mod tests;

#[derive(Debug, thiserror::Error)]
enum SizedBundleError {
    #[error("bundle does not have enough space left for the given sequence action")]
    NotEnoughSpace(SequenceAction),
    #[error("sequence action is larger than the max bundle size")]
    SequenceActionTooLarge(SequenceAction),
}

/// A bundle sequence actions to be submitted to the sequencer. Maintains the total size of the
/// bytes pushed to it and enforces a max size in bytes passed in the constructor. If an incoming
/// `seq_action` won't fit in the buffer it is flushed and a new bundle is started.
struct SizedBundle {
    /// The buffer of actions
    buffer: Vec<Action>,
    /// The current size of the bundle in bytes. This is equal to the sum of the size of the
    /// `seq_action`s + `ROLLUP_ID_LEN` for each.
    curr_size: usize,
    /// The max bundle size in bytes to enforce.
    max_size: usize,
}

impl SizedBundle {
    /// Create a new empty bundle with the given max size.
    fn new(max_size: usize) -> Self {
        Self {
            buffer: vec![],
            curr_size: 0,
            max_size,
        }
    }

    /// Buffer `seq_action` into the bundle. If the bundle won't fit `seq_action`, flush `buffer`,
    /// returning it, and start building up a new buffer using `seq_action`.
    fn push(&mut self, seq_action: SequenceAction) -> Result<(), SizedBundleError> {
        let seq_action_size = estimate_size_of_sequence_action(&seq_action);

        if seq_action_size > self.max_size {
            return Err(SizedBundleError::SequenceActionTooLarge(seq_action));
        }

        if self.curr_size + seq_action_size > self.max_size {
            return Err(SizedBundleError::NotEnoughSpace(seq_action));
        }

        self.buffer.push(Action::Sequence(seq_action));
        self.curr_size += seq_action_size;
        Ok(())
    }

    /// Replace self with a new empty bundle, returning the old bundle.
    fn flush(&mut self) -> SizedBundle {
        mem::replace(self, Self::new(self.max_size))
    }

    /// Consume self and return the underlying buffer of actions.
    fn into_actions(self) -> Vec<Action> {
        self.buffer
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum BundleFactoryError {
    #[error("sequence action is larger than the max bundle size. seq_action size: {size}")]
    SequenceActionTooLarge { size: usize, max_size: usize },
}

/// Manages the bundling of sequence actions into `SizedBundle`s. A `Vec<Action>` is flushed and
/// added to the `finished` queue when an incoming `SequenceAction` won't fit in the current bundle.
/// The `finished` queue operates in FIFO order, where `Vec<Action>`s are added to the back and
/// taken off from the front.
pub(super) struct BundleFactory {
    /// The current bundle being built.
    curr_bundle: SizedBundle,
    /// The queue of bundles that have been built but not yet sent to the sequencer.
    finished: VecDeque<SizedBundle>,
}

impl BundleFactory {
    pub(super) fn new(max_bytes_per_bundle: usize) -> Self {
        Self {
            curr_bundle: SizedBundle::new(max_bytes_per_bundle),
            finished: VecDeque::new(),
        }
    }

    /// Buffer `seq_action` into the current bundle. If the bundle won't fit `seq_action`, flush
    /// `curr_bundle` into the `finished` queue and start a new bundle
    pub(super) fn try_push(
        &mut self,
        seq_action: SequenceAction,
    ) -> Result<(), BundleFactoryError> {
        let seq_action_size = estimate_size_of_sequence_action(&seq_action);

        match self.curr_bundle.push(seq_action) {
            Err(SizedBundleError::SequenceActionTooLarge(_seq_action)) => {
                // reject the sequence action if it is larger than the max bundle size
                Err(BundleFactoryError::SequenceActionTooLarge {
                    size: seq_action_size,
                    max_size: self.curr_bundle.max_size,
                })
            }
            Err(SizedBundleError::NotEnoughSpace(seq_action)) => {
                // if the bundle is full, flush it and start a new one
                self.finished.push_back(self.curr_bundle.flush());
                self.curr_bundle
                    .push(seq_action)
                    .expect("seq_action should not be larger than max bundle size, this is a bug");
                Ok(())
            }
            Ok(()) => {
                trace!(
                    new_bundle_size = self.curr_bundle.curr_size,
                    seq_action_size = seq_action_size,
                    "bundled new sequence action"
                );
                Ok(())
            }
        }
    }

    /// Returns a handle to the next finished bundle if it exists.
    ///
    /// The bundle is only removed from the factory on calling [`NextFinishedBundle::pop`].
    /// This method primarily exists to work around async cancellation.
    pub(super) fn next_finished(&mut self) -> Option<NextFinishedBundle> {
        if self.finished.is_empty() {
            None
        } else {
            Some(NextFinishedBundle {
                bundle_factory: self,
            })
        }
    }

    /// Immediately the currently aggregating bundle.
    ///
    /// Returns an empty bundle if there are no bundled transactions.
    pub(super) fn pop_now(&mut self) -> Vec<Action> {
        self.finished
            .pop_front()
            .or_else(|| Some(self.curr_bundle.flush()))
            .map(SizedBundle::into_actions)
            .unwrap_or_default()
    }
}

pub(super) struct NextFinishedBundle<'a> {
    bundle_factory: &'a mut BundleFactory,
}

impl<'a> NextFinishedBundle<'a> {
    pub(super) fn pop(self) -> Vec<Action> {
        self.bundle_factory
            .finished
            .pop_front()
            .map(SizedBundle::into_actions)
            .expect("next bundle exists. this is a bug.")
    }
}

/// The size of the `seq_action` in bytes, including the rollup id.
fn estimate_size_of_sequence_action(seq_action: &SequenceAction) -> usize {
    seq_action.data.len() + ROLLUP_ID_LEN
}
