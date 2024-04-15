/// ! This module is responsible for bundling sequence actions into bundles that can be
/// submitted to the sequencer.
use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    mem,
};

use astria_core::sequencer::v1::{
    transaction::{
        action::SequenceAction,
        Action,
    },
    RollupId,
    ROLLUP_ID_LEN,
};
use serde::ser::{
    Serialize,
    SerializeStruct as _,
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

pub(super) struct SizedBundleReport<'a>(pub(super) &'a SizedBundle);

impl<'a> Serialize for SizedBundleReport<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut report = serializer.serialize_struct("SizedBundleReport", 2)?;
        report.serialize_field("size", &self.0.curr_size)?;
        report.serialize_field("rollup_counts", &self.0.rollup_counts)?;
        report.end()
    }
}

/// A bundle sequence actions to be submitted to the sequencer. Maintains the total size of the
/// bytes pushed to it and enforces a max size in bytes passed in the constructor. If an incoming
/// `seq_action` won't fit in the buffer it is flushed and a new bundle is started.
#[derive(Clone)]
pub(super) struct SizedBundle {
    /// The buffer of actions
    buffer: Vec<Action>,
    /// The current size of the bundle in bytes. This is equal to the sum of the size of the
    /// `seq_action`s + `ROLLUP_ID_LEN` for each.
    curr_size: usize,
    /// The max bundle size in bytes to enforce.
    max_size: usize,
    /// Mapping of rollup id to the number of sequence actions for that rollup id in the bundle.
    rollup_counts: HashMap<RollupId, usize>,
}

impl SizedBundle {
    /// Create a new empty bundle with the given max size.
    fn new(max_size: usize) -> Self {
        Self {
            buffer: vec![],
            curr_size: 0,
            max_size,
            rollup_counts: HashMap::new(),
        }
    }

    /// Buffer `seq_action` into the bundle. Requires calling `push_check` first on `seq_action` to
    /// validate its size
    ///
    /// # Errors
    /// - `seq_action` is beyond the max size allowed for the entire bundle
    /// - `seq_action` does not fit in the remaining space in the bundle
    fn try_push(&mut self, seq_action: SequenceAction) -> Result<(), SizedBundleError> {
        let seq_action_size = estimate_size_of_sequence_action(&seq_action);

        if seq_action_size > self.max_size {
            return Err(SizedBundleError::SequenceActionTooLarge(seq_action));
        }

        if self.curr_size + seq_action_size > self.max_size {
            return Err(SizedBundleError::NotEnoughSpace(seq_action));
        }

        self.rollup_counts
            .entry(seq_action.rollup_id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.buffer.push(Action::Sequence(seq_action));
        self.curr_size += seq_action_size;

        Ok(())
    }

    /// Replace self with a new empty bundle, returning the old bundle.
    fn flush(&mut self) -> SizedBundle {
        mem::replace(self, Self::new(self.max_size))
    }

    /// Consume self and return the underlying buffer of actions.
    pub(super) fn into_actions(self) -> Vec<Action> {
        self.buffer
    }

    /// Returns true if the bundle is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum BundleFactoryError {
    #[error("sequence action is larger than the max bundle size. seq_action size: {size}")]
    SequenceActionTooLarge { size: usize, max_size: usize },
    #[error(
        "finished bundle queue is full and the sequence action does not fit in the current bundle"
    )]
    FinishedQueueFull {
        curr_bundle_size: usize,
        finished_queue_capacity: usize,
        seq_action: SequenceAction,
    },
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
    /// Max amount of `SizedBundle`s that can be in the `finished` queue.
    finished_queue_capacity: usize,
    /// Indicates whether the `BundleFactory` is at capacity. Specifically, if the `finished`
    /// queue is full and an attempt to push a `SequenceAction` into `curr_bundle` returns
    /// `SizedBundleError::NotEnoughSpace`. Popping a bundle using `pop_now` or
    /// `NextFinishedBundle` will set the factory to not full.
    full: bool,
}

impl BundleFactory {
    pub(super) fn new(max_bytes_per_bundle: usize, finished_queue_capacity: usize) -> Self {
        Self {
            curr_bundle: SizedBundle::new(max_bytes_per_bundle),
            finished: VecDeque::new(),
            finished_queue_capacity,
            full: false,
        }
    }

    /// Buffer `seq_action` into the current bundle. If the bundle won't fit `seq_action`, flush
    /// `curr_bundle` into the `finished` queue and start a new bundle, unless the `finished` queue
    /// is at capacity.
    pub(super) fn try_push(
        &mut self,
        seq_action: SequenceAction,
    ) -> Result<(), BundleFactoryError> {
        let seq_action_size = estimate_size_of_sequence_action(&seq_action);

        match self.curr_bundle.try_push(seq_action) {
            Err(SizedBundleError::SequenceActionTooLarge(_seq_action)) => {
                // reject the sequence action if it is larger than the max bundle size
                Err(BundleFactoryError::SequenceActionTooLarge {
                    size: seq_action_size,
                    max_size: self.curr_bundle.max_size,
                })
            }
            Err(SizedBundleError::NotEnoughSpace(seq_action)) => {
                if self.finished.len() >= self.finished_queue_capacity {
                    Err(BundleFactoryError::FinishedQueueFull {
                        curr_bundle_size: self.curr_bundle.curr_size,
                        finished_queue_capacity: self.finished_queue_capacity,
                        seq_action,
                    })
                } else {
                    self.finished.push_back(self.curr_bundle.flush());
                    // if the finished queue is full after the flush mark it as such
                    if self.finished.len() == self.finished_queue_capacity {
                        self.full = true;
                    }

                    self.curr_bundle.try_push(seq_action).expect(
                        "seq_action should not be larger than max bundle size, this is a bug",
                    );
                    trace!(
                        new_bundle_size = self.curr_bundle.curr_size,
                        seq_action_size = seq_action_size,
                        finished_queue.current_size = self.finished.len(),
                        finished_queue.capacity = self.finished_queue_capacity,
                        "created new bundle and bundled new sequence action"
                    );
                    Ok(())
                }
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
    pub(super) fn pop_now(&mut self) -> SizedBundle {
        let bundle = self
            .finished
            .pop_front()
            .or_else(|| Some(self.curr_bundle.flush()))
            .unwrap_or(SizedBundle::new(self.curr_bundle.max_size));
        self.full = false;
        bundle
    }

    pub(super) fn is_full(&self) -> bool {
        self.full
    }
}

pub(super) struct NextFinishedBundle<'a> {
    bundle_factory: &'a mut BundleFactory,
}

impl<'a> NextFinishedBundle<'a> {
    pub(super) fn pop(self) -> SizedBundle {
        let bundle = self
            .bundle_factory
            .finished
            .pop_front()
            .expect("next bundle exists. this is a bug.");
        self.bundle_factory.full = false;
        bundle
    }
}

/// The size of the `seq_action` in bytes, including the rollup id.
fn estimate_size_of_sequence_action(seq_action: &SequenceAction) -> usize {
    seq_action.data.len() + ROLLUP_ID_LEN
}
