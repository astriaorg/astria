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
    eyre,
    Context,
    OptionExt,
};
use bytes::Bytes;
use prost::Message as _;

pub(crate) mod block_commitment_stream;
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

    pub(crate) fn into_raw(self) -> raw_sequencer_block::FilteredSequencerBlock {
        self.filtered_sequencer_block.into_raw()
    }

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

        let serialized_transactions = rollup_transactions
            .swap_remove(&rollup_id)
            .ok_or_eyre(
                "FilteredSequencerBlock does not contain transactions for the given rollup",
            )?
            .into_parts();

        let transactions = serialized_transactions
            .transactions
            .into_iter()
            .map(raw_sequencer_block::RollupData::decode)
            .collect::<Result<_, _>>()
            .wrap_err("failed to decode RollupData")?;

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
    block: execution::v1::Block,
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
            .hash()
            .as_ref()
            .try_into()
            .expect("rollup block hash must be 32 bytes")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Commitment {
    sequencer_height: u64,
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

    pub(crate) fn into_raw(self) -> raw_optimistic_block::SequencerBlockCommit {
        raw_optimistic_block::SequencerBlockCommit {
            height: self.sequencer_height,
            block_hash: Bytes::copy_from_slice(&self.sequnecer_block_hash),
        }
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
    pub(crate) fn with_optimistic(optimistic_block: Optimistic) -> Self {
        Self {
            optimistic: optimistic_block,
            executed: None,
            commitment: None,
        }
    }

    pub(crate) fn execute(&mut self, executed_block: Executed) -> eyre::Result<()> {
        if executed_block.sequencer_block_hash() != self.optimistic.sequencer_block_hash() {
            return Err(eyre!("block hash mismatch"));
        }

        self.executed = Some(executed_block);
        Ok(())
    }

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
}
