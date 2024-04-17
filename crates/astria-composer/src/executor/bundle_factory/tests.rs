#[cfg(test)]
mod sized_bundle_tests {
    use astria_core::{
        primitive::v1::{
            asset::default_native_asset_id,
            RollupId,
            ROLLUP_ID_LEN,
        },
        protocol::transaction::v1alpha1::action::SequenceAction,
    };
    use insta::{
        assert_json_snapshot,
        Settings,
    };

    use crate::executor::bundle_factory::{
        estimate_size_of_sequence_action,
        SizedBundle,
        SizedBundleError,
    };

    #[test]
    fn push_works() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };

        assert_eq!(estimate_size_of_sequence_action(&seq_action), 100);
        bundle.push(seq_action).unwrap();
    }

    #[test]
    fn push_seq_action_too_large() {
        // create a bundle with 100 bytes of max space
        let mut bundle = SizedBundle::new(100);

        // push a sequence action that is >100 bytes total
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN + 1],
            fee_asset_id: default_native_asset_id(),
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
            fee_asset_id: default_native_asset_id(),
        };
        bundle.push(initial_seq_action).unwrap();

        // push another sequence action that won't fit as the bundle is full but is less than max
        // size
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 1],
            fee_asset_id: default_native_asset_id(),
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
            fee_asset_id: default_native_asset_id(),
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

    fn snapshot_bundle() -> SizedBundle {
        let mut bundle = SizedBundle::new(200);
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 50 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        let seq_action1_2 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 50 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        let seq_action2 = SequenceAction {
            rollup_id: RollupId::new([2; ROLLUP_ID_LEN]),
            data: vec![2; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        bundle.push(seq_action1).unwrap();
        bundle.push(seq_action1_2).unwrap();
        bundle.push(seq_action2).unwrap();
        bundle
    }

    #[test]
    fn snapshots() {
        let bundle = snapshot_bundle();

        let mut settings = Settings::new();
        settings.set_sort_maps(true);

        settings.bind(|| {
            assert_json_snapshot!(bundle.rollup_counts);
        });
    }
}

#[cfg(test)]
mod bundle_factory_tests {
    use astria_core::{
        primitive::v1::{
            asset::default_native_asset_id,
            RollupId,
            ROLLUP_ID_LEN,
        },
        protocol::transaction::v1alpha1::action::SequenceAction,
    };

    use crate::executor::bundle_factory::{
        estimate_size_of_sequence_action,
        BundleFactory,
        BundleFactoryError,
    };

    #[test]
    fn try_push_works_no_flush() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
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
            fee_asset_id: default_native_asset_id(),
        };
        let actual_size = estimate_size_of_sequence_action(&seq_action);

        assert!(matches!(
            bundle_factory.try_push(seq_action),
            Err(BundleFactoryError::SequenceActionTooLarge {
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
            fee_asset_id: default_native_asset_id(),
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
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
    fn pop_finished_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total so it doesn't flush
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty
        assert_eq!(bundle_factory.finished.len(), 0);
        // assert `pop_finished()` returns an empty vec
        let next_bundle = bundle_factory.next_finished();
        assert!(next_bundle.is_none());
    }

    #[test]
    fn pop_now_finished_empty() {
        // create a bundle factory with max bundle size as 100 bytes
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total so it doesn't flush
        let seq_action = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        bundle_factory.try_push(seq_action.clone()).unwrap();

        // assert that the finished queue is empty (curr wasnt flushed)
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
        let mut bundle_factory = BundleFactory::new(100);

        // push a sequence action that is 100 bytes total
        let seq_action0 = SequenceAction {
            rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
            data: vec![0; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
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
            fee_asset_id: default_native_asset_id(),
        };
        bundle_factory.try_push(seq_action0.clone()).unwrap();

        // push another sequence action that is <100 bytes total to force the current bundle to
        // flush
        let seq_action1 = SequenceAction {
            rollup_id: RollupId::new([1; ROLLUP_ID_LEN]),
            data: vec![1; 100 - ROLLUP_ID_LEN],
            fee_asset_id: default_native_asset_id(),
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
}
