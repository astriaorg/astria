use anyhow::anyhow;

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
    pub(crate) fn new(cometbft_max_size: usize) -> Self {
        BlockSizeConstraints {
            max_sequencer: MAX_SEQUENCE_DATA_BYTES_PER_BLOCK,
            max_cometbft: cometbft_max_size,
            current_sequencer: 0,
            current_cometbft: 0,
        }
    }

    pub(crate) fn sequencer_has_space(&self, size: usize) -> bool {
        self.current_sequencer + size <= self.max_sequencer
    }

    pub(crate) fn cometbft_has_space(&self, size: usize) -> bool {
        self.current_cometbft + size <= self.max_cometbft
    }

    pub(crate) fn sequencer_checked_add(&mut self, size: usize) -> anyhow::Result<()> {
        if self.current_sequencer + size > self.max_sequencer {
            return Err(anyhow!("max sequencer size reached"));
        }
        self.current_sequencer += size;
        Ok(())
    }

    pub(crate) fn cometbft_checked_add(&mut self, size: usize) -> anyhow::Result<()> {
        if self.current_cometbft + size > self.max_cometbft {
            return Err(anyhow!("max cometbft size reached"));
        }
        self.current_cometbft += size;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cometbft_checks() {
        let mut block_size_constraints = BlockSizeConstraints::new(10);
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
        let mut block_size_constraints = BlockSizeConstraints::new(10);
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
