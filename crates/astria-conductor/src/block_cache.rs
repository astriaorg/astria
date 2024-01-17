//! A cache of sequencer blocks that are only yielded in sequential order.
use std::collections::BTreeMap;

use sequencer_client::{
    tendermint::block::Height,
    SequencerBlock,
};

pub(crate) trait GetSequencerHeight {
    fn get_height(&self) -> Height;
}

impl GetSequencerHeight for SequencerBlock {
    fn get_height(&self) -> Height {
        self.height()
    }
}

#[derive(Debug)]
pub(crate) struct BlockCache<T> {
    inner: BTreeMap<Height, T>,
    next_height: Height,
}

impl<T> BlockCache<T> {
    /// Creates a new block cache.
    ///
    /// The first block it will serve is at height 0.
    pub(crate) fn new() -> Self {
        Self::with_next_height(Height::from(0u32))
    }

    /// Creates a new block cache that starts at `next_height`.
    pub(crate) fn with_next_height(next_height: Height) -> Self {
        Self {
            inner: BTreeMap::new(),
            next_height,
        }
    }
}

impl<T: GetSequencerHeight> BlockCache<T> {
    /// Inserts a block using the height recorded in its header.
    ///
    /// Return an error if a block already exists at that height.
    pub(crate) fn insert(&mut self, block: T) -> Result<(), Error> {
        use std::collections::btree_map::Entry;
        let block_height = block.get_height();
        if block_height < self.next_height {
            return Err(Error::Old {
                block_height,
                current_height: self.next_height,
            });
        }
        match self.inner.entry(block_height) {
            Entry::Vacant(entry) => {
                entry.insert(block);
                Ok(())
            }
            Entry::Occupied(_) => Err(Error::Occupied {
                height: block_height,
            }),
        }
    }

    /// Return a handle to the next block in the cache.
    ///
    /// This method exists to make fetching the next block async cancellation safe.
    pub(crate) fn next_block(&mut self) -> Option<NextBlock<'_, T>> {
        if self.inner.contains_key(&self.next_height) {
            Some(NextBlock {
                cache: self,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("block at sequencer height {height} already in cache")]
    Occupied { height: Height },
    #[error(
        "block too old: expect sequencer height {current_height} or newer, got {block_height}"
    )]
    Old {
        block_height: Height,
        current_height: Height,
    },
}

#[must_use = "the next block must be popped from the handle to be useful"]
pub(crate) struct NextBlock<'a, T> {
    cache: &'a mut BlockCache<T>,
}

impl<'a, T> NextBlock<'a, T> {
    pub(crate) fn pop(self) -> T {
        let Self {
            cache,
        } = self;
        let block = cache
            .inner
            .remove(&cache.next_height)
            .expect("the block exists; this is a bug");
        cache.next_height = cache.next_height.increment();
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyBlock {
        height: Height,
    }

    impl From<u32> for DummyBlock {
        fn from(height: u32) -> DummyBlock {
            DummyBlock {
                height: height.into(),
            }
        }
    }

    impl From<Height> for DummyBlock {
        fn from(height: Height) -> DummyBlock {
            DummyBlock {
                height,
            }
        }
    }

    impl GetSequencerHeight for DummyBlock {
        fn get_height(&self) -> Height {
            self.height
        }
    }

    fn make_cache() -> BlockCache<DummyBlock> {
        BlockCache::<DummyBlock>::new()
    }

    #[test]
    fn empty_cache_gives_no_block() {
        let mut cache = make_cache();
        assert!(cache.next_block().is_none())
    }

    #[test]
    fn blocks_are_returned_in_order() {
        let mut cache = make_cache();
        cache.insert(0u32.into()).unwrap();
        cache.insert(1u32.into()).unwrap();
        cache.insert(2u32.into()).unwrap();
        assert_eq!(0, cache.next_block().unwrap().pop().height.value());
        assert_eq!(1, cache.next_block().unwrap().pop().height.value());
        assert_eq!(2, cache.next_block().unwrap().pop().height.value());
        assert!(cache.next_block().is_none());
    }

    #[test]
    fn blocks_at_same_height_are_rejected() {
        let mut cache = make_cache();
        cache.insert(1u32.into()).unwrap();
        assert!(cache.insert(1u32.into()).is_err());
    }

    #[test]
    fn old_blocks_are_rejected() {
        let mut cache = make_cache();
        cache.insert(0u32.into()).unwrap();
        cache.insert(1u32.into()).unwrap();
        cache.next_block().unwrap().pop();
        cache.next_block().unwrap().pop();
        assert!(cache.insert(1u32.into()).is_err());
    }

    #[test]
    fn hole_leads_to_no_block() {
        let mut cache = make_cache();
        cache.insert(0u32.into()).unwrap();
        cache.insert(2u32.into()).unwrap();
        assert_eq!(0, cache.next_block().unwrap().pop().height.value());
        assert!(cache.next_block().is_none());
        cache.insert(1u32.into()).unwrap();
        assert_eq!(1, cache.next_block().unwrap().pop().height.value());
        assert_eq!(2, cache.next_block().unwrap().pop().height.value());
        assert!(cache.next_block().is_none());
    }
}
