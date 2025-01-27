use astria_core::{
    execution,
    generated::astria::{
        auction::v1alpha1 as auction,
        sequencerblock::v1 as raw_sequencer_block,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::{
        block::{
            self,
            FilteredSequencerBlock,
            FilteredSequencerBlockParts,
        },
        RollupTransactions,
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
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
    ) -> eyre::Result<auction::BaseBlock> {
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

        Ok(auction::BaseBlock {
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
        raw: auction::ExecuteOptimisticBlockStreamResponse,
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

    pub(crate) fn rollup_block_hash(&self) -> RollupBlockHash {
        RollupBlockHash::new(self.block.hash().clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RollupBlockHash(Bytes);

impl RollupBlockHash {
    #[must_use]
    pub(crate) fn new(inner: Bytes) -> Self {
        Self(inner)
    }
}

impl std::fmt::Display for RollupBlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::STANDARD,
        };

        if f.alternate() {
            Base64Display::new(&self.0, &STANDARD).fmt(f)?;
        } else {
            for byte in &self.0 {
                write!(f, "{byte:02x}")?;
            }
        }
        Ok(())
    }
}
