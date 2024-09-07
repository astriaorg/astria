mod address_bytes;
mod address_prefix;
mod balance;
mod block_hash;
mod block_height;
mod block_timestamp;
mod chain_id;
mod deposit;
mod fee;
mod ibc_prefixed_denom;
mod nonce;
mod proof;
mod revision_number;
mod rollup_id;
mod rollup_ids;
mod rollup_transactions;
mod sequencer_block_header;
mod storage_version;
mod trace_prefixed_denom;
mod transaction_hash;
mod validator_set;

use anyhow::{
    anyhow,
    Context,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(crate) use self::{
    address_bytes::AddressBytes,
    address_prefix::AddressPrefix,
    balance::Balance,
    block_hash::BlockHash,
    block_height::BlockHeight,
    block_timestamp::BlockTimestamp,
    chain_id::ChainId,
    deposit::Deposit,
    fee::Fee,
    ibc_prefixed_denom::IbcPrefixedDenom,
    nonce::Nonce,
    proof::Proof,
    revision_number::RevisionNumber,
    rollup_id::RollupId,
    rollup_ids::RollupIds,
    rollup_transactions::RollupTransactions,
    sequencer_block_header::SequencerBlockHeader,
    storage_version::StorageVersion,
    trace_prefixed_denom::TracePrefixedDenom,
    transaction_hash::TransactionHash,
    validator_set::ValidatorSet,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum StoredValue<'a> {
    ChainId(ChainId<'a>),
    RevisionNumber(RevisionNumber),
    StorageVersion(StorageVersion),
    AddressBytes(AddressBytes<'a>),
    Balance(Balance),
    Nonce(Nonce),
    Fee(Fee),
    AddressPrefix(AddressPrefix<'a>),
    IbcPrefixedDenom(IbcPrefixedDenom<'a>),
    TracePrefixedDenom(TracePrefixedDenom<'a>),
    RollupId(RollupId<'a>),
    RollupIds(RollupIds<'a>),
    Deposit(Deposit<'a>),
    ValidatorSet(ValidatorSet<'a>),
    BlockHash(BlockHash<'a>),
    BlockHeight(BlockHeight),
    BlockTimestamp(BlockTimestamp),
    SequencerBlockHeader(SequencerBlockHeader<'a>),
    RollupTransactions(RollupTransactions<'a>),
    Proof(Proof<'a>),
    TransactionHash(TransactionHash<'a>),
    Unit,
}

impl<'a> StoredValue<'a> {
    pub(crate) fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        borsh::to_vec(&self).context("failed to serialize stored value")
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        borsh::from_slice(bytes).context("failed to deserialize stored value")
    }
}

fn type_mismatch(expected: &'static str, found: &StoredValue) -> anyhow::Error {
    let found = match found {
        StoredValue::ChainId(_) => "chain id",
        StoredValue::RevisionNumber(_) => "revision number",
        StoredValue::StorageVersion(_) => "storage version",
        StoredValue::AddressBytes(_) => "address bytes",
        StoredValue::Balance(_) => "balance",
        StoredValue::Nonce(_) => "nonce",
        StoredValue::Fee(_) => "fee",
        StoredValue::AddressPrefix(_) => "address prefix",
        StoredValue::IbcPrefixedDenom(_) => "ibc-prefixed denom",
        StoredValue::TracePrefixedDenom(_) => "trace-prefixed denom",
        StoredValue::RollupId(_) => "rollup id",
        StoredValue::RollupIds(_) => "rollup ids",
        StoredValue::Deposit(_) => "deposit",
        StoredValue::ValidatorSet(_) => "validator set",
        StoredValue::BlockHash(_) => "block hash",
        StoredValue::BlockHeight(_) => "block height",
        StoredValue::BlockTimestamp(_) => "block timestamp",
        StoredValue::SequencerBlockHeader(_) => "sequencer block header",
        StoredValue::RollupTransactions(_) => "rollup transactions",
        StoredValue::Proof(_) => "proof",
        StoredValue::TransactionHash(_) => "transaction hash",
        StoredValue::Unit => "unit",
    };
    anyhow!("type mismatch: expected {expected}, found {found}")
}
