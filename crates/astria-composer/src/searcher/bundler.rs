use std::{
    collections::VecDeque,
    future::{
        self,
        Ready,
    },
    mem,
};

use astria_core::sequencer::v1alpha1::{
    transaction::{
        action::SequenceAction,
        Action,
    },
    ROLLUP_ID_LEN,
};
use tracing::debug;

#[derive(Debug, thiserror::Error)]
enum SizedBundleError {
    #[error("bundle does not have enough space left for the given sequence action")]
    NotEnoughSpace(SequenceAction),
}

/// A bundle sequence actions to be submitted to the sequencer. Maintains the total size of the
/// bytes pushed to it and enforces a max size in bytes passed in the constructor. If an incoming
/// `seq_action` won't fit in the buffer it is flushed and a new bundle is started.
pub(super) struct SizedBundle {
    /// The buffer of actions
    pub(super) buffer: Vec<Action>,
    /// The current size of the bundle in bytes. This is equal to the sum of the size of the
    /// `seq_action`s + `ROLLUP_ID_LEN` for each.
    pub(super) curr_size: usize,
    /// The max bundle size in bytes to enforce.
    max_size: usize,
}

impl SizedBundle {
    /// Create a new empty bundle with the given max size.
    pub(super) fn new(max_size: usize) -> Self {
        Self {
            buffer: vec![],
            curr_size: 0,
            max_size,
        }
    }

    /// Buffer `seq_action` into the bundle. If the bundle won't fit `seq_action`, flush `buffer`,
    /// returning it, and start building up a new buffer using `seq_action`.
    fn push(&mut self, seq_action: SequenceAction) -> Result<(), SizedBundleError> {
        let seq_action_size = seq_action.data.len() + ROLLUP_ID_LEN;
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
    pub(super) fn into_actions(self) -> Vec<Action> {
        self.buffer
    }

    pub(super) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.buffer.len()
    }
}

/// Manages the bundling of sequence actions into `SizedBundle`s. A `SizedBundle` is flushed and
/// added to the `finished` queue when an incoming `SequenceAction` won't fit in the current bundle.
/// The `finished` queue operates in FIFO order, where `SizedBundle`s are added to the back and
/// taken off from the front.
pub(super) struct Bundler {
    /// The current bundle being built.
    pub(super) curr_bundle: SizedBundle,
    /// The queue of bundles that have been built but not yet sent to the sequencer.
    pub(super) finished: VecDeque<SizedBundle>,
}

impl Bundler {
    pub(super) fn new(max_size: usize) -> Self {
        Self {
            curr_bundle: SizedBundle::new(max_size),
            finished: VecDeque::new(),
        }
    }

    /// Buffer `seq_action` into the current bundle. If the bundle won't fit `seq_action`, flush
    /// `curr_bundle` into the `finished` queue and start a new bundle
    pub(super) fn push(&mut self, seq_action: SequenceAction) {
        let seq_action_size = seq_action.data.len() + ROLLUP_ID_LEN;
        if let Err(SizedBundleError::NotEnoughSpace(seq_action)) = self.curr_bundle.push(seq_action)
        {
            // if the bundle is full, flush it and start a new one
            self.finished.push_back(self.curr_bundle.flush());
            self.curr_bundle
                .push(seq_action)
                .expect("seq_action should not be larger than max bundle size");
        }
        debug!(
            new_bundle_size = ?self.curr_bundle.curr_size,
            seq_action_size = ?seq_action_size,
            "bundled new sequence action"
        );
    }

    /// Get the next bundle from the `finished` queue. If the queue is empty, flush `curr_bundle`
    pub(super) fn get_next(&mut self) -> Ready<Option<SizedBundle>> {
        future::ready(self.finished.pop_front())
    }

    /// Flush the current bundle into the `finished` queue. A bundle can be preempted if there are
    /// no pending `finished` and the `curr_bundle` is not empty.
    pub(super) fn flush_curr_bundle(&mut self) {
        if !self.curr_bundle.is_empty() && self.finished.is_empty() {
            debug!(
                bundle_size = self.curr_bundle.curr_size,
                "bundler preempting current bundle to the finished queue"
            );
            self.finished.push_back(self.curr_bundle.flush());
        }
    }
}
