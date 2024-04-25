use std::{
    collections::HashSet,
    ops::RangeInclusive,
};

use ::astria_core::generated::execution::v1alpha2::Block as RawBlock;
use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    generated::{
        execution::{
            v1alpha2 as raw,
            v1alpha2::execution_service_client::ExecutionServiceClient,
        },
        sequencerblock::v1alpha1::RollupData,
    },
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use bytes::Bytes;
use pbjson_types::Timestamp;
use tonic::transport::Channel;
use tracing::instrument;

/// A newtype wrapper around [`ExecutionServiceClient`] to work with
/// idiomatic types.
#[derive(Clone)]
pub(crate) struct Client {
    uri: tonic::transport::Uri,
    inner: ExecutionServiceClient<Channel>,
}

impl Client {
    #[instrument(skip_all, fields(rollup_uri = %uri))]
    pub(crate) async fn connect(uri: tonic::transport::Uri) -> eyre::Result<Self> {
        let inner = ExecutionServiceClient::connect(uri.clone())
            .await
            .wrap_err("failed constructing execution service client")?;
        Ok(Self {
            uri,
            inner,
        })
    }

    /// Issues the `astria.execution.v1alpha2.BatchGetBlocks` RPC for `block_numbers`.
    ///
    /// Returns a sequence of blocks sorted by block number and without duplicates,
    /// holes in the requested range, or blocks outside the requested range.
    ///
    /// Returns an error if dupliates, holes, or extra blocks are found.
    #[instrument(skip_all, fields(
        uri = %self.uri,
        from = block_numbers.start(),
        to = block_numbers.end(),
    ))]
    pub(crate) async fn batch_get_blocks(
        &mut self,
        block_numbers: RangeInclusive<u32>,
    ) -> eyre::Result<Vec<Block>> {
        fn identifier(number: u32) -> raw::BlockIdentifier {
            raw::BlockIdentifier {
                identifier: Some(raw::block_identifier::Identifier::BlockNumber(number)),
            }
        }
        let request = raw::BatchGetBlocksRequest {
            identifiers: block_numbers.clone().map(identifier).collect(),
        };
        let raw_blocks = self
            .inner
            .batch_get_blocks(request)
            .await
            .wrap_err("failed to execute batch get blocks RPC")?
            .into_inner()
            .blocks;
        let raw_blocks = ensure_batch_get_blocks_is_correct(raw_blocks, block_numbers).wrap_err(
            "received an incorrect response; did the rollup execution service violate the \
             batch-get-blocks contract?",
        )?;
        let blocks = raw_blocks
            .into_iter()
            .map(Block::try_from_raw)
            .collect::<Result<_, _>>()
            .wrap_err("failed validating received blocks")?;
        Ok(blocks)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetGenesisInfo`
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_genesis_info(&mut self) -> eyre::Result<GenesisInfo> {
        let request = raw::GetGenesisInfoRequest {};
        let response = self
            .inner
            .get_genesis_info(request)
            .await
            .wrap_err("failed to get genesis_info")?
            .into_inner();
        let genesis_info = GenesisInfo::try_from_raw(response)
            .wrap_err("failed converting raw response to validated genesis info")?;
        Ok(genesis_info)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.ExecuteBlock`
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash` - Block hash of the parent block
    /// * `transactions` - List of transactions extracted from the sequencer block
    /// * `timestamp` - Optional timestamp of the sequencer block
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(super) async fn execute_block(
        &mut self,
        prev_block_hash: Bytes,
        transactions: Vec<Vec<u8>>,
        timestamp: Timestamp,
    ) -> eyre::Result<Block> {
        use prost::Message;

        let transactions = transactions
            .into_iter()
            .map(|tx| RollupData::decode(tx.as_slice()))
            .collect::<Result<_, _>>()
            .wrap_err("failed to decode tx bytes as RollupData")?;

        let request = raw::ExecuteBlockRequest {
            prev_block_hash,
            transactions,
            timestamp: Some(timestamp),
        };
        let response = self
            .inner
            .execute_block(request)
            .await
            .wrap_err("failed to execute block")?
            .into_inner();
        let block = Block::try_from_raw(response)
            .wrap_err("failed converting raw response to validated block")?;
        Ok(block)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.GetCommitmentState`
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(crate) async fn get_commitment_state(&mut self) -> eyre::Result<CommitmentState> {
        let request = raw::GetCommitmentStateRequest {};
        let response = self
            .inner
            .get_commitment_state(request)
            .await
            .wrap_err("failed to get commitment state")?
            .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }

    /// Calls remote procedure `astria.execution.v1alpha2.UpdateCommitmentState`
    ///
    /// # Arguments
    ///
    /// * `firm` - The firm block
    /// * `soft` - The soft block
    #[instrument(skip_all, fields(uri = %self.uri))]
    pub(super) async fn update_commitment_state(
        &mut self,
        commitment_state: CommitmentState,
    ) -> eyre::Result<CommitmentState> {
        let request = raw::UpdateCommitmentStateRequest {
            commitment_state: Some(commitment_state.into_raw()),
        };
        let response = self
            .inner
            .update_commitment_state(request)
            .await
            .wrap_err("failed to update commitment state")?
            .into_inner();
        let commitment_state = CommitmentState::try_from_raw(response)
            .wrap_err("failed converting raw response to validated commitment state")?;
        Ok(commitment_state)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum BatchGetBlocksError {
    #[error(
        "duplicates blocks numbers: [{}]",
        itertools::join(.0, ", "),
    )]
    Dupes(HashSet<u32>),
    #[error(
        "missing blocks for numbers: [{}]",
        itertools::join(.0, ", "),
    )]
    Holes(HashSet<u32>),
    #[error(
        "extra blocks for numbers: [{}]",
        itertools::join(.0, ", "),
    )]
    Extras(HashSet<u32>),
}

fn ensure_batch_get_blocks_is_correct(
    mut blocks: Vec<RawBlock>,
    requested_numbers: RangeInclusive<u32>,
) -> Result<Vec<RawBlock>, BatchGetBlocksError> {
    blocks.sort_unstable_by_key(|block| block.number);
    let mut dupes = HashSet::new();
    blocks.dedup_by(|a, b| {
        let same = a.number == b.number;
        if same {
            dupes.insert(a.number);
        };
        same
    });
    if !dupes.is_empty() {
        return Err(BatchGetBlocksError::Dupes(dupes));
    }
    let mut holes = requested_numbers.collect::<HashSet<_>>();
    let mut extras = HashSet::new();
    for block in &blocks {
        if !holes.remove(&block.number) {
            extras.insert(block.number);
        }
    }
    if !holes.is_empty() {
        return Err(BatchGetBlocksError::Holes(holes));
    }
    if !extras.is_empty() {
        return Err(BatchGetBlocksError::Extras(extras));
    }
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use maplit::hashset;

    use super::{
        ensure_batch_get_blocks_is_correct,
        RawBlock,
    };
    use crate::executor::client::BatchGetBlocksError;

    fn block(number: u32) -> RawBlock {
        RawBlock {
            number,
            ..RawBlock::default()
        }
    }

    #[test]
    fn correct_batched_blocks_is_returned_sorted() {
        let range = 2..=7;
        let expected: Vec<_> = range.clone().map(block).collect();
        let blocks: Vec<_> = range.clone().rev().map(block).collect();
        assert_eq!(
            Ok(expected),
            ensure_batch_get_blocks_is_correct(blocks, range),
        );
    }

    #[test]
    fn batched_blocks_with_dupes_are_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks.push(block(5));
        blocks.push(block(3));
        assert_eq!(
            Err(BatchGetBlocksError::Dupes(hashset! {3, 5})),
            ensure_batch_get_blocks_is_correct(blocks, range),
        );
    }

    #[test]
    fn batched_blocks_with_holes_are_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks.swap_remove(2); // index 2 => block number 4
        blocks.swap_remove(2); // index 2 => block number 7
        assert_eq!(
            Err(BatchGetBlocksError::Holes(hashset! {4, 7})),
            ensure_batch_get_blocks_is_correct(blocks, range),
        );
    }

    #[test]
    fn batched_blocks_with_extras_are_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks.push(block(8));
        blocks.push(block(9));
        assert_eq!(
            Err(BatchGetBlocksError::Extras(hashset! {8, 9})),
            ensure_batch_get_blocks_is_correct(blocks, range),
        );
    }
}
