use astria_core::{
    primitive::v1::{
        RollupId,
        ROLLUP_ID_LEN,
    },
    protocol::transaction::v1alpha1::action::SequenceAction,
};

mod sized_bundle {
    use crate::{
        executor::bundle_factory::{
            SizedBundle,
            SizedBundleError,
        },
        test_utils::{
            empty_sequence_action,
            sequence_action_of_max_size,
            sequence_action_with_n_bytes,
        },
    };

    #[test]
    fn push_works() {
        let mut bundle = SizedBundle::new(200);

        let seq_action = sequence_action_of_max_size(200);
        bundle.try_push(seq_action).unwrap();
    }

    #[test]
    fn push_seq_action_too_large() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(200);

        // push an action with > 200 bytes. the proto encoding will guarantee
        // to take us over 200.
        let seq_action = sequence_action_with_n_bytes(200);
        assert!(matches!(
            bundle.try_push(seq_action),
            Err(SizedBundleError::SequenceActionTooLarge(_))
        ));
    }

    #[test]
    fn push_not_enough_space() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(200);

        // push a sequence action that is 100 bytes total
        bundle.try_push(sequence_action_of_max_size(200)).unwrap();

        assert!(matches!(
            bundle.try_push(empty_sequence_action()),
            Err(SizedBundleError::NotEnoughSpace(actual_seq_action))
            if actual_seq_action.rollup_id == empty_sequence_action().rollup_id
            && actual_seq_action.data == empty_sequence_action().data
        ));
    }

    #[test]
    fn flush_sanity_check() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(200);

        // push a sequence action successfully
        let seq_action = sequence_action_of_max_size(200);
        bundle.try_push(seq_action.clone()).unwrap();

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
mod bundle_factory {
    use super::*;
    use crate::{
        executor::bundle_factory::{
            BundleFactory,
            BundleFactoryError,
        },
        test_utils::{
            sequence_action_of_max_size,
            sequence_action_with_n_bytes,
        },
    };

    #[test]
    fn try_push_works_no_flush() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action).unwrap();

        // assert that the bundle factory has no bundles in the finished queue
        assert!(bundle_factory.finished.is_empty());
    }

    #[test]
    fn try_push_seq_action_too_large() {
        let mut bundle_factory = BundleFactory::new(200, 10);

        // push an action with > 200 bytes. the proto encoding will guarantee
        // to take us over 200.
        let seq_action = sequence_action_with_n_bytes(200);

        assert!(matches!(
            bundle_factory.try_push(seq_action),
            Err(BundleFactoryError::SequenceActionTooLarge { .. })
        ));
    }

    #[test]
    fn try_push_flushes_and_pop_finished_works() {
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action0 = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = sequence_action_of_max_size(150);
        bundle_factory.try_push(seq_action1).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);
        // assert `pop_finished()` will return `seq_action0`
        let next_actions = bundle_factory.next_finished();
        let actions = next_actions.unwrap().pop().into_actions();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);
    }

    #[test]
    fn try_push_full_sanity_check() {
        let mut bundle_factory = BundleFactory::new(200, 1);

        // push a sequence action that is 100 bytes total
        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // try to push a third bundle that wouldn't fit in `curr_bundle`, forcing the factory to
        // flush it into `finished` this shouldn't work since the `finished` queue's
        // capacity is 1.
        let err = bundle_factory
            .try_push(seq_action.clone())
            .expect_err("the action should be rejected");

        // assert that the bundle factory has one bundle in the finished queue, that the factory is
        // full and that err was returned
        // allow: this is intended to match all possible variants
        #[allow(clippy::match_wildcard_for_single_variants)]
        match err {
            BundleFactoryError::FinishedQueueFull(_) => {}
            other => panic!("expected a FinishedQueueFull variant, but got {other:?}"),
        }
        assert_eq!(bundle_factory.finished.len(), 1);
        assert!(bundle_factory.is_full());
    }

    #[test]
    fn pop_finished_empty() {
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_finished()` returns an empty vec
        let next_bundle = bundle_factory.next_finished();
        assert!(next_bundle.is_none());
    }

    #[test]
    fn pop_finished_no_longer_full() {
        let mut bundle_factory = BundleFactory::new(200, 1);

        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // push another sequence action to force the current bundle to flush
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // try to push a third bundle that wouldn't fit in `curr_bundle`, forcing the factory to
        // flush it into `finished` this shouldn't work since the `finished` queue's
        // capacity is 1.
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            ..sequence_action_of_max_size(200)
        };
        let err = bundle_factory
            .try_push(seq_action1.clone())
            .expect_err("the action should have been rejected");

        // assert that the bundle factory has one bundle in the finished queue, that the factory is
        // full and that err was returned
        // allow: this is intended to match all possible variants
        #[allow(clippy::match_wildcard_for_single_variants)]
        match err {
            BundleFactoryError::FinishedQueueFull(_) => {}
            other => panic!("expected a FinishedQueueFull variant, but got {other:?}"),
        }
        assert_eq!(bundle_factory.finished.len(), 1);
        assert!(bundle_factory.is_full());

        // assert `next_finished().pop()` will change the status back to not full
        let _next_bundle = bundle_factory.next_finished().unwrap().pop();
        assert_eq!(bundle_factory.finished.len(), 0);
        assert!(!bundle_factory.is_full());
    }

    #[test]
    fn pop_now_finished_empty() {
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty (curr wasn't flushed)
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_now()` returns `seq_action`
        let actions = bundle_factory.pop_now().into_actions();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action.data);
    }

    #[test]
    fn pop_now_finished_not_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action0 = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            ..sequence_action_of_max_size(200)
        };
        bundle_factory.try_push(seq_action1).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);
        // assert `pop_now()` will return `seq_action0`
        let actions = bundle_factory.pop_now().into_actions();
        let actual_seq_action = actions[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);
    }

    #[test]
    fn pop_now_all_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100, 10);

        // assert that the finished queue is empty
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_now()` returns an empty vec
        let actions = bundle_factory.pop_now();
        assert!(actions.is_empty());
    }

    #[test]
    fn pop_now_finished_then_curr_then_empty() {
        let mut bundle_factory = BundleFactory::new(200, 10);

        let seq_action0 = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            ..sequence_action_of_max_size(200)
        };
        bundle_factory.try_push(seq_action1.clone()).unwrap();

        // assert that the bundle factory has one bundle in the finished queue
        assert_eq!(bundle_factory.finished.len(), 1);

        // assert `pop_now()` will return `seq_action0` on the first call
        let actions_finished = bundle_factory.pop_now().into_actions();
        assert_eq!(actions_finished.len(), 1);
        let actual_seq_action = actions_finished[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action0.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action0.data);

        // assert that the finished queue is empty now
        assert_eq!(bundle_factory.finished.len(), 0);

        // assert `pop_now()` will return `seq_action1` on the second call (i.e. from curr)
        let actions_curr = bundle_factory.pop_now().into_actions();
        assert_eq!(actions_curr.len(), 1);
        let actual_seq_action = actions_curr[0].as_sequence().unwrap();
        assert_eq!(actual_seq_action.rollup_id, seq_action1.rollup_id);
        assert_eq!(actual_seq_action.data, seq_action1.data);

        // assert the third call will return an empty vec
        let actions_empty = bundle_factory.pop_now();
        assert!(actions_empty.is_empty());
    }

    #[test]
    fn pop_now_full() {
        let mut bundle_factory = BundleFactory::new(200, 1);

        // push a sequence action that is 100 bytes total
        let seq_action = sequence_action_of_max_size(200);
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // push another sequence action that is to force the current bundle to flush
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert `pop_now()` will set the factory to no longer full
        let _actions_finished = bundle_factory.pop_now();
        assert_eq!(bundle_factory.finished.len(), 0);
        assert!(!bundle_factory.is_full());
    }
}
