use astria_eyre::eyre::{
    ensure,
    eyre,
    OptionExt as _,
    Result,
    WrapErr,
};

use super::commitment::GeneratedCommitments;

/// The maximum number of bytes allowed in sequencer action data.
const MAX_SEQUENCE_DATA_BYTES_PER_BLOCK: usize = 256_000;

/// Struct for organizing block size constraints in prepare proposal
#[derive(Copy, Clone, serde::Serialize)]
pub(crate) struct BlockSizeConstraints {
    max_size_sequencer: usize,
    max_size_cometbft: usize,
    current_size_sequencer: usize,
    current_size_cometbft: usize,
}

impl BlockSizeConstraints {
    pub(crate) fn new(cometbft_max_size: i64, uses_data_item_enum: bool) -> Result<Self> {
        let cometbft_max_size = usize::try_from(cometbft_max_size)
            .wrap_err("failed to convert cometbft_max_size to usize")?;
        let commitments_size = if uses_data_item_enum {
            GeneratedCommitments::<true>::total_size()
        } else {
            GeneratedCommitments::<false>::total_size()
        };
        if cometbft_max_size < commitments_size {
            return Err(eyre!(
                "cometbft_max_size must be at least GeneratedCommitments::TOTAL_SIZE"
            ));
        }

        Ok(BlockSizeConstraints {
            max_size_sequencer: MAX_SEQUENCE_DATA_BYTES_PER_BLOCK,
            max_size_cometbft: cometbft_max_size,
            current_size_sequencer: 0,
            current_size_cometbft: commitments_size,
        })
    }

    pub(crate) fn new_unlimited_cometbft() -> Self {
        BlockSizeConstraints {
            max_size_sequencer: MAX_SEQUENCE_DATA_BYTES_PER_BLOCK,
            max_size_cometbft: usize::MAX,
            current_size_sequencer: 0,
            current_size_cometbft: GeneratedCommitments::<true>::total_size(),
        }
    }

    pub(crate) fn sequencer_has_space(&self, size: usize) -> bool {
        size <= self
            .max_size_sequencer
            .saturating_sub(self.current_size_sequencer)
    }

    pub(crate) fn cometbft_has_space(&self, size: usize) -> bool {
        size <= self
            .max_size_cometbft
            .saturating_sub(self.current_size_cometbft)
    }

    pub(crate) fn sequencer_checked_add(&mut self, size: usize) -> Result<()> {
        let new_size = self
            .current_size_sequencer
            .checked_add(size)
            .ok_or_eyre("overflow adding to sequencer size")?;
        ensure!(
            new_size <= self.max_size_sequencer,
            "max sequencer size reached"
        );
        self.current_size_sequencer = new_size;
        Ok(())
    }

    pub(crate) fn cometbft_checked_add(&mut self, size: usize) -> Result<()> {
        let new_size = self
            .current_size_cometbft
            .checked_add(size)
            .ok_or_eyre("overflow adding to cometBFT size")?;
        ensure!(
            new_size <= self.max_size_cometbft,
            "max cometBFT size reached"
        );
        self.current_size_cometbft = new_size;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cometbft_checks() {
        let mut block_size_constraints = BlockSizeConstraints::new(
            10 + i64::try_from(GeneratedCommitments::<true>::total_size()).unwrap(),
            true,
        )
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
        let mut block_size_constraints = BlockSizeConstraints::new(
            i64::try_from(GeneratedCommitments::<true>::total_size()).unwrap(),
            true,
        )
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
