mod block_hash;
mod extended_commit_info;
mod proof;
mod rollup_ids;
mod rollup_transactions;
mod sequencer_block_header;
mod upgrade_change_hashes;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::grpc) use self::{
    block_hash::BlockHash,
    extended_commit_info::ExtendedCommitInfo,
    proof::Proof,
    rollup_ids::RollupIds,
    rollup_transactions::RollupTransactions,
    sequencer_block_header::SequencerBlockHeader,
    upgrade_change_hashes::UpgradeChangeHashes,
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
    UpgradeChangeHashes(UpgradeChangeHashes<'a>),
    ExtendedCommitInfo(ExtendedCommitInfo<'a>),
}
