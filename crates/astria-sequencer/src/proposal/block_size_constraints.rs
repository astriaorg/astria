use anyhow::anyhow;

use super::commitment::GeneratedCommitments;

/// The maximum number of bytes allowed in sequencer action data.
const MAX_SEQUENCE_DATA_BYTES_PER_BLOCK: usize = 256_000;

/// Struct for organizing block size constraints in prepare proposal
pub(crate) struct BlockSizeConstraints {
    pub(crate) max_sequencer: usize,
    pub(crate) max_cometbft: usize,
    pub(crate) current_sequencer: usize,
    pub(crate) current_cometbft: usize,
}

impl BlockSizeConstraints {
    pub(crate) fn new(cometbft_max_size: usize) -> anyhow::Result<Self> {
        if cometbft_max_size < GeneratedCommitments::TOTAL_SIZE {
            return Err(anyhow!(
                "cometbft_max_size must be at least GeneratedCommitments::TOTAL_SIZE"
            ));
        }

        Ok(BlockSizeConstraints {
            max_sequencer: MAX_SEQUENCE_DATA_BYTES_PER_BLOCK,
            max_cometbft: cometbft_max_size,
            current_sequencer: 0,
            current_cometbft: GeneratedCommitments::TOTAL_SIZE,
        })
    }

    pub(crate) fn sequencer_has_space(&self, size: usize) -> bool {
        size <= self.max_sequencer.saturating_sub(self.current_sequencer)
    }

    pub(crate) fn cometbft_has_space(&self, size: usize) -> bool {
        size <= self.max_cometbft.saturating_sub(self.current_cometbft)
    }

    pub(crate) fn sequencer_checked_add(&mut self, size: usize) -> anyhow::Result<()> {
        if self.current_sequencer.saturating_add(size) > self.max_sequencer {
            return Err(anyhow!("max sequencer size reached"));
        }
        self.current_sequencer = self
            .current_sequencer
            .checked_add(size)
            .expect("overflow in adding to sequencer size, shouldn't happen");
        Ok(())
    }

    pub(crate) fn cometbft_checked_add(&mut self, size: usize) -> anyhow::Result<()> {
        if self.current_cometbft.saturating_add(size) > self.max_cometbft {
            return Err(anyhow!("max cometbft size reached"));
        }
        self.current_cometbft = self
            .current_cometbft
            .checked_add(size)
            .expect("overflow in adding to cometbft size, shouldn't happen");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cometbft_checks() {
        let mut block_size_constraints =
            BlockSizeConstraints::new(10 + GeneratedCommitments::TOTAL_SIZE)
                .expect("should be able to create block constraints with this size");
        assert!(
            block_size_constraints.cometbft_has_space(10),
            "cometBFT has space"
        );
        assert!(
            !block_size_constraints.cometbft_has_space(11),
            "cometBFT doesn't have space"
        );
        assert!(
            block_size_constraints.cometbft_checked_add(10).is_ok(),
            "should be able to grow to cometBFT max size"
        );
        assert!(
            block_size_constraints.cometbft_checked_add(1).is_err(),
            "shouldn't be able to grow past cometBFT max size"
        );
    }

    #[test]
    fn sequencer_checks() {
        let mut block_size_constraints =
            BlockSizeConstraints::new(GeneratedCommitments::TOTAL_SIZE)
                .expect("should be able to create block constraints with this size");
        assert!(
            block_size_constraints.sequencer_has_space(MAX_SEQUENCE_DATA_BYTES_PER_BLOCK),
            "sequencer has space"
        );
        assert!(
            !block_size_constraints.sequencer_has_space(MAX_SEQUENCE_DATA_BYTES_PER_BLOCK + 1),
            "sequencer doesn't have space"
        );
        assert!(
            block_size_constraints
                .sequencer_checked_add(MAX_SEQUENCE_DATA_BYTES_PER_BLOCK)
                .is_ok(),
            "should be able to grow to sequencer max size"
        );
        assert!(
            block_size_constraints.sequencer_checked_add(1).is_err(),
            "shouldn't be able to grow past sequencer max size"
        );
    }
}
