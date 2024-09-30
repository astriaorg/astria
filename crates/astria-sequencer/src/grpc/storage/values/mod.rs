mod block_hash;
mod proof;
mod rollup_ids;
mod rollup_transactions;
mod sequencer_block_header;

use std::fmt::{
    self,
    Display,
    Formatter,
};

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

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ValueImpl::RollupIds(rollup_ids) => write!(f, "rollup_ids {rollup_ids}"),
            ValueImpl::BlockHash(block_hash) => write!(f, "block hash {block_hash}"),
            ValueImpl::SequencerBlockHeader(header) => {
                write!(f, "sequencer block header at height {}", header.height())
            }
            ValueImpl::RollupTransactions(txs) => {
                write!(f, "rollup transactions for rollup {}", txs.rollup_id())
            }
            ValueImpl::Proof(_proof) => write!(f, "proof"),
        }
    }
}
