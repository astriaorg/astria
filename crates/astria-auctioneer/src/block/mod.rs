use astria_core::{
    Protobuf,
    execution,
    generated::astria::{
        bundle::v1alpha1 as raw_bundle,
        sequencerblock::v1 as raw_sequencer_block,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::{
        RollupTransactions,
        block::{
            self,
            FilteredSequencerBlock,
            FilteredSequencerBlockParts,
        },
    },
};
use astria_eyre::eyre::{
    self,
    Context,
    eyre,
};
use bytes::Bytes;
use prost::Message as _;

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
    pub(crate) fn new(filtered_sequencer_block: FilteredSequencerBlock) -> Self {
        Self {
            filtered_sequencer_block,
        }
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
            .map(RollupTransactions::into_parts);

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
            sequencer_block_hash: Bytes::copy_from_slice(block_hash.as_bytes()),
            transactions,
            timestamp,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Executed {
    /// The rollup block metadata that resulted from executing the optimistic block.
    block: execution::v1::Block,
    /// The hash of the sequencer block that was executed optimistically.
    sequencer_block_hash: block::Hash,
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

    pub(crate) fn sequencer_block_hash(&self) -> &block::Hash {
        &self.sequencer_block_hash
    }

    pub(crate) fn rollup_block_hash(&self) -> [u8; 32] {
        self.block
            .hash()
            .as_ref()
            .try_into()
            .expect("rollup block hash must be 32 bytes")
    }
}
