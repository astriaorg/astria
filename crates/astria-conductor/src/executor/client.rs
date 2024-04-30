use std::ops::RangeInclusive;

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
    ensure,
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
        let request = raw::BatchGetBlocksRequest {
            identifiers: block_numbers.clone().map(block_identifier).collect(),
        };
        let raw_blocks = self
            .inner
            .batch_get_blocks(request)
            .await
            .wrap_err("failed to execute batch get blocks RPC")?
            .into_inner()
            .blocks;
        ensure_batch_get_blocks_is_correct(&raw_blocks, block_numbers).wrap_err(
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

    #[instrument(skip_all, fields(uri = %self.uri), err)]
    pub(crate) async fn get_block(&mut self, block_number: u32) -> eyre::Result<Block> {
        let request = raw::GetBlockRequest {
            identifier: Some(block_identifier(block_number)),
        };
        let raw_block = self
            .inner
            .get_block(request)
            .await
            .wrap_err("failed to execute astria.execution.v1alpha2.GetBlocks RPC")?
            .into_inner();
        ensure!(
            block_number == raw_block.number,
            "requested block at number `{block_number}`, but received block contained `{}`",
            raw_block.number
        );
        Block::try_from_raw(raw_block).wrap_err("failed validating received block")
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
        "number of returned blocks does not match requested; requested: `{expected}`, received: \
         `{actual}`"
    )]
    LengthOfResponse { expected: u32, actual: u32 },
    #[error(
        "returned blocks did not match in the sequence they were requested at: [{}]",
        itertools::join(.0, ", ")
    )]
    MismatchedBlocks(Vec<MismatchedBlock>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MismatchedBlock {
    index: usize,
    requested: u32,
    got: u32,
}

impl std::fmt::Display for MismatchedBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            index,
            requested,
            got,
        } = self;
        f.write_fmt(format_args!(
            "{{\"index\": {index}, \"requested\": {requested}, \"got\": {got}}}"
        ))
    }
}

fn ensure_batch_get_blocks_is_correct(
    blocks: &[RawBlock],
    requested_numbers: RangeInclusive<u32>,
) -> Result<(), BatchGetBlocksError> {
    let expected = requested_numbers
        .end()
        .saturating_sub(*requested_numbers.start())
        .saturating_add(1);
    // allow: the calling function can only fetch up to u32::MAX blocks, which itself
    // is unrealistic. If the next check passes due to a u64 being truncated to u32::MAX
    // then something is going very wrong and will be caught at the latest in the next step.
    #[allow(clippy::cast_possible_truncation)]
    let actual = blocks.len() as u32;
    if expected != actual {
        return Err(BatchGetBlocksError::LengthOfResponse {
            expected,
            actual,
        });
    }
    let mut mismatched = Vec::with_capacity(blocks.len());
    for (index, (requested, got)) in requested_numbers
        .zip(blocks.iter().map(|block| block.number))
        .enumerate()
    {
        if requested != got {
            mismatched.push(MismatchedBlock {
                index,
                requested,
                got,
            });
        }
    }
    if !mismatched.is_empty() {
        return Err(BatchGetBlocksError::MismatchedBlocks(mismatched));
    }
    Ok(())
}

/// Utility function to construct a `astria.execution.v1alpha2.BlockIdentifier` from `number`
/// to use in RPC requests.
fn block_identifier(number: u32) -> raw::BlockIdentifier {
    raw::BlockIdentifier {
        identifier: Some(raw::block_identifier::Identifier::BlockNumber(number)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_batch_get_blocks_is_correct,
        MismatchedBlock,
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
    fn mismatched_block_is_formatted_as_expected() {
        let block = MismatchedBlock {
            index: 1,
            requested: 2,
            got: 3,
        };

        assert_eq!(
            r#"{"index": 1, "requested": 2, "got": 3}"#,
            block.to_string()
        );
    }

    #[test]
    fn expected_batch_response_passes() {
        let range = 2..=7;
        let blocks: Vec<_> = range.clone().map(block).collect();
        ensure_batch_get_blocks_is_correct(&blocks, range).unwrap();
    }

    #[test]
    fn too_long_batch_response_is_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks.push(block(8));
        assert_eq!(
            Err(BatchGetBlocksError::LengthOfResponse {
                expected: 6,
                actual: 7,
            }),
            ensure_batch_get_blocks_is_correct(&blocks, range),
        );
    }

    #[test]
    fn too_short_batch_response_is_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks.pop();
        assert_eq!(
            Err(BatchGetBlocksError::LengthOfResponse {
                expected: 6,
                actual: 5,
            }),
            ensure_batch_get_blocks_is_correct(&blocks, range),
        );
    }

    #[test]
    fn mismatched_batch_response_is_caught() {
        let range = 2..=7;
        let mut blocks: Vec<_> = range.clone().map(block).collect();
        blocks[2].number = 8;
        blocks[4].number = 9;
        assert_eq!(
            Err(BatchGetBlocksError::MismatchedBlocks(vec![
                MismatchedBlock {
                    index: 2,
                    requested: 4,
                    got: 8
                },
                MismatchedBlock {
                    index: 4,
                    requested: 6,
                    got: 9
                },
            ])),
            ensure_batch_get_blocks_is_correct(&blocks, range),
        );
    }
}
