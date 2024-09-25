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

pub(crate) use self::{
    block_hash::BlockHash,
    proof::Proof,
    rollup_ids::RollupIds,
    rollup_transactions::RollupTransactions,
    sequencer_block_header::SequencerBlockHeader,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum Value<'a> {
    RollupIds(RollupIds<'a>),
    BlockHash(BlockHash<'a>),
    SequencerBlockHeader(SequencerBlockHeader<'a>),
    RollupTransactions(RollupTransactions<'a>),
    Proof(Proof<'a>),
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::RollupIds(rollup_ids) => write!(f, "rollup_ids {rollup_ids}"),
            Value::BlockHash(block_hash) => write!(f, "block hash {block_hash}"),
            Value::SequencerBlockHeader(header) => {
                write!(f, "sequencer block header at height {}", header.height())
            }
            Value::RollupTransactions(txs) => {
                write!(f, "rollup transactions for rollup {}", txs.rollup_id())
            }
            Value::Proof(_proof) => write!(f, "proof"),
        }
    }
}
