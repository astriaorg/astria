mod block_hash;
mod proof;
mod rollup_ids;
mod rollup_transactions;
mod sequencer_block_header;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::grpc) use self::{
    block_hash::BlockHash,
    proof::Proof,
    rollup_ids::RollupIds,
    rollup_transactions::RollupTransactions,
    sequencer_block_header::SequencerBlockHeader,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    RollupIds(RollupIds<'a>),
    BlockHash(BlockHash<'a>),
    SequencerBlockHeader(SequencerBlockHeader<'a>),
    RollupTransactions(RollupTransactions<'a>),
    Proof(Proof<'a>),
}
