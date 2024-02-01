use celestia_client::celestia_types::Height as CelestiaHeight;
use sequencer_client::tendermint::block::Height as SequencerHeight;

/// A necessary evil because the celestia client code uses a forked tendermint-rs.
pub(crate) trait IncrementableHeight {
    fn increment(self) -> Self;
}

impl IncrementableHeight for CelestiaHeight {
    fn increment(self) -> Self {
        self.increment()
    }
}

impl IncrementableHeight for SequencerHeight {
    fn increment(self) -> Self {
        self.increment()
    }
}

/// A poor man's inclusive range to avoid converting heights to/from integers.
pub(crate) fn height_range_inclusive<T>(start: T, end: T) -> impl Iterator<Item = T>
where
    T: IncrementableHeight + PartialOrd + Copy,
{
    let start = (start <= end).then_some(start);
    std::iter::successors(start, move |&height| {
        let next_height = height.increment();
        (next_height <= end).then_some(next_height)
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn inclusive_range_from_gives_1_to_3_gives_1_to_3() {
        let start: SequencerHeight = 1u32.into();
        let end = 3u32.into();
        let mut range = height_range_inclusive(start, end);
        assert_eq!(Some(1u32.into()), range.next());
        assert_eq!(Some(2u32.into()), range.next());
        assert_eq!(Some(3u32.into()), range.next());
        assert!(range.next().is_none());
    }

    #[test]
    fn inclusive_range_from_1_to_1_gives_1() {
        let start: SequencerHeight = 1u32.into();
        let mut range = height_range_inclusive(start, start);
        assert_eq!(Some(1u32.into()), range.next());
        assert!(range.next().is_none());
    }

    #[test]
    fn inclusive_range_from_1_to_0_is_empty() {
        let start: SequencerHeight = 1u32.into();
        let end: SequencerHeight = 0u32.into();
        let mut range = height_range_inclusive(start, end);
        assert!(range.next().is_none());
    }
}
