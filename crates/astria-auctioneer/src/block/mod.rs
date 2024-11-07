use astria_core::{
    execution,
    generated::{
        bundle::v1alpha1 as raw_bundle,
        sequencerblock::{
            optimisticblock::v1alpha1 as raw_optimistic_block,
            v1 as raw_sequencer_block,
        },
    },
    primitive::v1::RollupId,
    sequencerblock::v1::block::{
        FilteredSequencerBlock,
        FilteredSequencerBlockParts,
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    Context,
};
use bytes::Bytes;
use prost::Message as _;
use telemetry::display::base64;

use crate::bundle::Bundle;

pub(crate) mod commitment_stream;
pub(crate) mod executed_stream;
pub(crate) mod optimistic_stream;

/// Converts a [`tendermint::Time`] to a [`prost_types::Timestamp`].
fn convert_tendermint_time_to_protobuf_timestamp(
    value: sequencer_client::tendermint::Time,
) -> pbjson_types::Timestamp {
    let sequencer_client::tendermint_proto::google::protobuf::Timestamp {
        seconds,
        nanos,
    } = value.into();
    pbjson_types::Timestamp {
        seconds,
        nanos,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Optimistic {
    /// The optimistic block data, filtered for a rollup id.
    filtered_sequencer_block: FilteredSequencerBlock,
}

impl Optimistic {
    pub(crate) fn try_from_raw(
        raw: raw_sequencer_block::FilteredSequencerBlock,
    ) -> eyre::Result<Self> {
        Ok(Self {
            filtered_sequencer_block: FilteredSequencerBlock::try_from_raw(raw)?,
        })
    }

    /// Converts this [`Optimistic`] into a [`BaseBlock`] for the given `rollup_id`.
    /// If there are no transactions for the given `rollup_id`, this will return a `BaseBlock`
    /// with no transactions.
    ///
    /// # Errors
    /// Invalid `RollupData` included in the optimistic block data will result in an error.
    // TODO: add typed errors here?
    pub(crate) fn try_into_base_block(
        self,
        rollup_id: RollupId,
    ) -> eyre::Result<raw_bundle::BaseBlock> {
        let FilteredSequencerBlockParts {
            block_hash,
            header,
            mut rollup_transactions,
            ..
        } = self.filtered_sequencer_block.into_parts();

        let maybe_serialized_transactions = rollup_transactions
            .swap_remove(&rollup_id)
            .map(|transactions| transactions.into_parts());

        let transactions =
            maybe_serialized_transactions.map_or(Ok(vec![]), |serialized_transactions| {
                serialized_transactions
                    .transactions
                    .into_iter()
                    .map(raw_sequencer_block::RollupData::decode)
                    .collect::<Result<_, _>>()
                    .wrap_err("failed to decode RollupData")
            })?;

        let timestamp = Some(convert_tendermint_time_to_protobuf_timestamp(header.time()));

        Ok(raw_bundle::BaseBlock {
            sequencer_block_hash: Bytes::copy_from_slice(&block_hash),
            transactions,
            timestamp,
        })
    }

    pub(crate) fn sequencer_block_hash(&self) -> [u8; 32] {
        self.filtered_sequencer_block.block_hash().clone()
    }

    pub(crate) fn sequencer_height(&self) -> u64 {
        self.filtered_sequencer_block.height().into()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Executed {
    /// The rollup block metadata that resulted from executing the optimistic block.
    block: execution::v1::Block,
    /// The hash of the sequencer block that was executed optimistically.
    sequencer_block_hash: [u8; 32],
}

impl Executed {
    pub(crate) fn try_from_raw(
        raw: raw_bundle::ExecuteOptimisticBlockStreamResponse,
    ) -> eyre::Result<Self> {
        let block = if let Some(raw_block) = raw.block {
            execution::v1::Block::try_from_raw(raw_block).wrap_err("invalid rollup block")?
        } else {
            return Err(eyre!("missing block"));
        };

        let sequencer_block_hash = raw
            .base_sequencer_block_hash
            .as_ref()
            .try_into()
            .wrap_err("invalid block hash")?;

        Ok(Self {
            block,
            sequencer_block_hash,
        })
    }

    pub(crate) fn sequencer_block_hash(&self) -> [u8; 32] {
        self.sequencer_block_hash
    }

    pub(crate) fn parent_rollup_block_hash(&self) -> [u8; 32] {
        self.block
            .parent_block_hash()
            .as_ref()
            .try_into()
            .expect("rollup block hash must be 32 bytes")
    }

    pub(crate) fn rollup_block_hash(&self) -> [u8; 32] {
        self.block
            .hash()
            .as_ref()
            .try_into()
            .expect("rollup block hash must be 32 bytes")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Commitment {
    /// The height of the sequencer block that was committed.
    sequencer_height: u64,
    /// The hash of the sequencer block that was committed.
    sequnecer_block_hash: [u8; 32],
}

impl Commitment {
    pub(crate) fn try_from_raw(
        raw: raw_optimistic_block::SequencerBlockCommit,
    ) -> eyre::Result<Self> {
        Ok(Self {
            sequencer_height: raw.height,
            sequnecer_block_hash: raw
                .block_hash
                .as_ref()
                .try_into()
                .wrap_err("invalid block hash")?,
        })
    }

    pub(crate) fn sequencer_block_hash(&self) -> [u8; 32] {
        self.sequnecer_block_hash
    }

    pub(crate) fn sequencer_height(&self) -> u64 {
        self.sequencer_height
    }
}

pub(crate) struct Current {
    optimistic: Optimistic,
    executed: Option<Executed>,
    commitment: Option<Commitment>,
}

impl Current {
    /// Creates a new `Current` with the given `optimistic_block`.
    pub(crate) fn with_optimistic(optimistic_block: Optimistic) -> Self {
        Self {
            optimistic: optimistic_block,
            executed: None,
            commitment: None,
        }
    }

    /// Updates the `Current` with the given `executed_block`.
    /// This will fail if the `executed_block` does not match the `optimistic_block`'s sequencer
    /// block hash.
    pub(crate) fn execute(&mut self, executed_block: Executed) -> eyre::Result<()> {
        if executed_block.sequencer_block_hash() != self.optimistic.sequencer_block_hash() {
            return Err(eyre!("block hash mismatch"));
        }

        self.executed = Some(executed_block);
        Ok(())
    }

    /// Updates the `Current` with the given `block_commitment`.
    /// This will fail if the `block_commitment` does not match the `optimistic_block`'s sequencer
    /// block hash.
    pub(crate) fn commitment(&mut self, block_commitment: Commitment) -> eyre::Result<()> {
        if block_commitment.sequencer_block_hash() != self.optimistic.sequencer_block_hash() {
            return Err(eyre!("block hash mismatch"));
        }
        if block_commitment.sequencer_height() != self.optimistic.sequencer_height() {
            return Err(eyre!("block height mismatch"));
        }

        self.commitment = Some(block_commitment);
        Ok(())
    }

    pub(crate) fn sequencer_block_hash(&self) -> [u8; 32] {
        self.optimistic.sequencer_block_hash()
    }

    pub(crate) fn rollup_parent_block_hash(&self) -> Option<[u8; 32]> {
        self.executed
            .as_ref()
            .map(|executed| executed.parent_rollup_block_hash())
    }

    /// Ensures that the given `bundle` is valid for the current block state.
    pub(crate) fn ensure_bundle_is_valid(&self, bundle: &Bundle) -> eyre::Result<()> {
        ensure!(
            bundle.base_sequencer_block_hash() == self.sequencer_block_hash(),
            "incoming bundle's sequencer block hash {bundle_hash} does not match current \
             sequencer block hash {current_hash}",
            bundle_hash = base64(bundle.base_sequencer_block_hash()),
            current_hash = base64(self.sequencer_block_hash())
        );

        if let Some(rollup_parent_block_hash) = self.rollup_parent_block_hash() {
            ensure!(
                bundle.parent_rollup_block_hash() == rollup_parent_block_hash,
                "bundle's rollup parent block hash {bundle_hash} does not match current rollup \
                 parent block hash {current_hash}",
                bundle_hash = base64(bundle.parent_rollup_block_hash()),
                current_hash = base64(rollup_parent_block_hash)
            );
        }

        Ok(())
    }
}
