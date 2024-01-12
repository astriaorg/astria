use std::{
    collections::VecDeque,
    mem,
};

use astria_core::sequencer::v1alpha1::{
    transaction::{
        action::SequenceAction,
        Action,
    },
    ROLLUP_ID_LEN,
};
use tracing::trace;

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
                    .expect("seq_action should not be larger than max bundle size");
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

    /// Get the next bundle from the `finished` queue.
    pub(super) fn pop_finished(&mut self) -> Vec<Action> {
        self.finished
            .pop_front()
            .map(SizedBundle::into_actions)
            .unwrap_or_default()
    }

    /// Get the next bundle from the `finished` queue. If the queue is empty, flush `curr_bundle`.
    pub(super) fn pop_now(&mut self) -> Vec<Action> {
        self.finished
            .pop_front()
            .or_else(|| Some(self.curr_bundle.flush()))
            .map(SizedBundle::into_actions)
            .unwrap_or_default()
    }
}

/// The size of the `seq_action` in bytes, including the rollup id.
fn estimate_size_of_sequence_action(seq_action: &SequenceAction) -> usize {
    seq_action.data.len() + ROLLUP_ID_LEN
}

#[cfg(test)]
mod sized_bundle_tests {
    use astria_core::sequencer::v1alpha1::{
        transaction::action::SequenceAction,
        RollupId,
        ROLLUP_ID_LEN,
    };

    use super::SizedBundle;
    use crate::searcher::bundle_factory::{
        estimate_size_of_sequence_action,
        SizedBundleError,
    };

    #[test]
    fn push_works() {
        // create a bundle with 100 bytes of max space
        let mut bundle = super::SizedBundle::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action = super::SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };

        assert_eq!(estimate_size_of_sequence_action(&seq_action), 100);
        bundle.push(seq_action).unwrap();
    }

    #[test]
    fn push_seq_action_too_large() {
        // create a bundle with 100 bytes of max space
        let mut bundle = super::SizedBundle::new(100);

        // push a sequence action that is >100 bytes total
        let seq_action = super::SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN + 1],
        };

        assert!(estimate_size_of_sequence_action(&seq_action) > 100);
        assert!(matches!(
            bundle.push(seq_action),
            Err(SizedBundleError::SequenceActionTooLarge(_))
        ));
    }

    #[test]
    fn push_not_enough_space() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(100);

        // push a sequence action that is 100 bytes total
        let initial_seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle.push(initial_seq_action).unwrap();

        // push another sequence action that won't fit as the bundle is full but is less than max
        // size
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 1],
        };

        assert!(estimate_size_of_sequence_action(&seq_action) < 100);
        assert!(matches!(
            bundle.push(seq_action.clone()),
            Err(SizedBundleError::NotEnoughSpace(actual_seq_action))
            if actual_seq_action.rollup_id == seq_action.rollup_id && actual_seq_action.data == seq_action.data
        ));
    }

    #[test]
    fn flush_sanity_check() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(100);

        // push a sequence action sucessfully
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
        };
        bundle.push(seq_action.clone()).unwrap();

        // flush the bundle
        let flushed_bundle = bundle.flush();

        // assert that the initial bundle is empty
        assert!(bundle.buffer.is_empty());

        // assert that the flushed bundle has just the sequence action pushed earlier
        let actions = flushed_bundle.into_actions();
        assert_eq!(actions.len(), 1);
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action.data);
    }
}

#[cfg(test)]
mod bundle_factory_tests {
    use astria_core::sequencer::v1alpha1::{
        transaction::action::SequenceAction,
        RollupId,
        ROLLUP_ID_LEN,
    };

    use super::BundleFactory;
    use crate::searcher::bundle_factory::estimate_size_of_sequence_action;

    #[test]
    fn try_push_works_no_flush() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action).unwrap();

        // assert that the bundle factory has no bundles in the finished queue
        assert!(bundle_factory.finished.is_empty());
    }

    #[test]
    fn try_push_seq_action_too_large() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is >100 bytes total
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN + 1],
        };
        let actual_size = estimate_size_of_sequence_action(&seq_action);

        assert!(matches!(
            bundle_factory.try_push(seq_action),
            Err(super::BundleFactoryError::SequenceActionTooLarge {
                size,
                max_size
            }) if size == actual_size && max_size == 100
        ));
    }

    #[test]
    fn try_push_flushes_and_pop_finished_works() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action1).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);
        // assert `pop_finished()` will return `seq_action0`
        let actions = bundle_factory.pop_finished();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);
    }

    #[test]
    fn pop_finished_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total so it doesn't flush
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_finished()` returns an empty vec
        let actions = bundle_factory.pop_finished();
        assert!(actions.is_empty());
    }

    #[test]
    fn pop_now_finished_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total so it doesn't flush
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty (curr wasnt flushed)
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_now()` returns `seq_action`
        let actions = bundle_factory.pop_now();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action.data);
    }

    #[test]
    fn pop_now_finished_not_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action1).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);
        // assert `pop_now()` will return `seq_action0`
        let actions = bundle_factory.pop_now();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);
    }

    #[test]
    fn pop_now_all_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // assert that the finished queue is empty
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_now()` returns an empty vec
        let actions = bundle_factory.pop_now();
        assert!(actions.is_empty());
    }

    #[test]
    fn pop_now_finished_then_curr_then_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
        };
        bundle_factory.try_push(seq_action1.clone()).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);

        // assert `pop_now()` will return `seq_action0` on the first call
        let actions_finished = bundle_factory.pop_now();
        assert_eq!(actions_finished.len(), 1);
        let actual_seq_action = actions_finished[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);

        // assert that the finished queue is empty now
        assert_eq!(bundle_factory.finished.len(), 0);

        // assert `pop_now()` will return `seq_action1` on the second call (i.e. from curr)
        let actions_curr = bundle_factory.pop_now();
        assert_eq!(actions_curr.len(), 1);
        let actual_seq_action = actions_curr[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action1.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action1.data);

        // assert the third call will return an empty vec
        let actions_empty = bundle_factory.pop_now();
        assert!(actions_empty.is_empty());
    }
}
