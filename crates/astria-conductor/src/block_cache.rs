//! A cache of sequencer blocks that are only yielded in sequential order.
use std::{
    collections::BTreeMap,
    future::Future,
};

use pin_project_lite::pin_project;
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
    inner: BTreeMap<u64, T>,
    next_height: u64,
    prev_height: u64,
}

impl<T> BlockCache<T> {
    /// Creates a new block cache that starts at `next_height`.
    pub(crate) fn with_next_height(next_height: Height) -> Result<Self, Error> {
        let next_height = next_height.value();
        if next_height == 0 {
            return Err(Error::ZeroHeightsNotSupported);
        }
        let prev_height = next_height - 1;
        Ok(Self {
            inner: BTreeMap::new(),
            next_height,
            prev_height,
        })
    }
}

impl<T> BlockCache<T> {
    /// Returns the next sequential block if it exists in the cache.
    pub(crate) fn pop(&mut self) -> Option<T> {
        let block = self.inner.remove(&self.next_height)?;
        self.next_height += 1;
        self.prev_height += 1;
        Some(block)
    }

    pub(crate) fn drop_obsolete(&mut self, latest_height: Height) {
        let latest_height = latest_height.value();
        self.next_height = std::cmp::max(self.next_height, latest_height);
        // Splitting the btree always involves an allocation, so only do it if necessary
        if self.inner.first_key_value().map(|(&height, _)| height) < Some(latest_height) {
            let only_non_obsolete = self.inner.split_off(&latest_height);
            self.inner = only_non_obsolete;
        }
    }

    /// Return a handle to the next block in the cache.
    ///
    /// This method exists to make fetching the next block async cancellation safe.
    pub(crate) fn next_block(&mut self) -> NextBlock<'_, T> {
        NextBlock {
            cache: self,
        }
    }
}

impl<T: GetSequencerHeight> BlockCache<T> {
    /// Inserts a block using the height recorded in its header.
    ///
    /// Return an error if a block already exists at that height.
    pub(crate) fn insert(&mut self, block: T) -> Result<(), Error> {
        use std::collections::btree_map::Entry;
        let block_height = block.get_height().value();
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

    /// Reschedules a block at the previous height.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// + the height of the block is not equal to the previous height recorded in the struct.
    /// + the height of the block is at level 0.
    pub(crate) fn reschedule_block(&mut self, block: T) -> Result<(), Error> {
        let block_height = block.get_height().value();
        if block_height == 0 {
            return Err(Error::ReschedulingZeroBlockHeight);
        }
        if block_height != self.prev_height {
            return Err(Error::ReschedulingWrongBlockHeight {
                block_height,
                prev_height: self.prev_height,
            });
        }
        self.prev_height -= 1;
        self.next_height -= 1;
        let res = self.inner.insert(block_height, block);
        assert!(res.is_none(), "no block must exist at the previous height");
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("block at sequencer height {height} already in cache")]
    Occupied { height: u64 },
    #[error(
        "block too old: expect sequencer height {current_height} or newer, got {block_height}"
    )]
    Old {
        block_height: u64,
        current_height: u64,
    },
    #[error("rescheduling blocks with height 0 are not permitted")]
    ReschedulingZeroBlockHeight,
    #[error(
        "can only reschdule blocks at the immediate previous height. Previous height \
         {prev_height}, block height {block_height}"
    )]
    ReschedulingWrongBlockHeight { block_height: u64, prev_height: u64 },
    #[error("starting heights of zero are not supported")]
    ZeroHeightsNotSupported,
}

pin_project! {
    pub(crate) struct NextBlock<'a, T> {
        cache: &'a mut BlockCache<T>,
    }
}

impl<'a, T> Future for NextBlock<'a, T> {
    type Output = Option<T>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        std::task::Poll::Ready((*this.cache).pop())
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
        BlockCache::<DummyBlock>::with_next_height(Height::from(1u32)).unwrap()
    }

    #[test]
    fn empty_cache_gives_no_block() {
        let mut cache = make_cache();
        assert!(cache.pop().is_none());
    }

    #[test]
    fn blocks_are_returned_in_order() {
        let mut cache = make_cache();
        cache.insert(1u32.into()).unwrap();
        cache.insert(2u32.into()).unwrap();
        cache.insert(3u32.into()).unwrap();
        assert_eq!(1, cache.pop().unwrap().height.value());
        assert_eq!(2, cache.pop().unwrap().height.value());
        assert_eq!(3, cache.pop().unwrap().height.value());
        assert!(cache.pop().is_none());
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
        cache.insert(1u32.into()).unwrap();
        cache.insert(2u32.into()).unwrap();
        cache.pop().unwrap();
        cache.pop().unwrap();
        assert!(cache.insert(2u32.into()).is_err());
    }

    #[test]
    fn hole_leads_to_no_block() {
        let mut cache = make_cache();
        cache.insert(1u32.into()).unwrap();
        cache.insert(3u32.into()).unwrap();
        assert_eq!(1, cache.pop().unwrap().height.value());
        assert!(cache.pop().is_none());
        cache.insert(2u32.into()).unwrap();
        assert_eq!(2, cache.pop().unwrap().height.value());
        assert_eq!(3, cache.pop().unwrap().height.value());
        assert!(cache.pop().is_none());
    }

    #[tokio::test]
    async fn awaited_next_block_pops_block() {
        let mut cache = make_cache();
        cache.insert(1u32.into()).unwrap();
        assert_eq!(1, cache.next_block().await.unwrap().height.value());
        assert!(cache.pop().is_none());
    }

    #[test]
    fn dropped_next_block_leaves_cache_unchanged() {
        let mut cache = make_cache();
        cache.insert(1u32.into()).unwrap();
        {
            let _fut = cache.next_block();
        }
        assert_eq!(1, cache.pop().unwrap().height.value());
    }
}
