# Data flow and verification

This document addresses how rollup data flows throughout the system and is
verified before execution.

## Background

The purpose of the Astria sequencer is to batch, order, and commit to data on
behalf of rollups. The entry point of data is transactions sent by users, and
the exit point is execution by a rollup node.

## Entry point

The entry point for rollup data is via a `RollupDataSubmission` action, which can
become part of a sequencer transaction. The data types are as follows:

```rust
pub struct RollupDataSubmission {
    rollup_id: RollupId,
    data: Bytes,
    // the asset to pay fees with
    fee_asset: asset::Denom,
}
```

```rust
// an unsigned transaction body
pub struct TransactionBody {
    actions: Actions, // vector of actions
    params: TransactionParams, // chain id, nonce
}
```

The `data` field inside the `RollupDataSubmission` is arbitrary bytes, which should
be an encoded rollup transaction. The sequencer is agnostic to the transaction
format of the rollups using it. The `rollup_id` field is an identifier for the
rollup the data is destined for.

To submit rollup data to the system, the user creates a transaction with a
`RollupDataSubmission` within it and signs and submits it to the sequencer. The
sequencer will then include it in a block, thus finalizing its ordering.

## Sequencer to data availability

Once a transaction (and the actions within it) is included in a sequencer block,
the block data is published via a data availability layer.

The block data published is as follows:

```rust
pub struct SequencerBlockHeader {
    chain_id: tendermint::chain::Id,
    height: tendermint::block::Height,
    time: Time,
    // the merkle root of all the rollup data in the block
    rollup_transactions_root: [u8; 32],
    // the merkle root of all transactions in the block
    data_hash: [u8; 32],
    proposer_address: account::Id,
}

pub struct SequencerBlock {
    /// the cometbft block hash for this block
    block_hash: [u8; 32],
    /// the block header, which contains cometbft header info and additional sequencer-specific
    /// commitments.
    header: SequencerBlockHeader,
    /// The collection of rollup transactions that were included in this block.
    rollup_transactions: IndexMap<RollupId, RollupTransactions>,
    /// The inclusion proof that the rollup transactions merkle root is included
    /// in `header.data_hash`.
    rollup_transactions_proof: merkle::Proof,
    /// The inclusion proof that the rollup IDs commitment is included
    /// in `header.data_hash`.
    rollup_ids_proof: merkle::Proof,
}
```

When the `SequencerBlock` is actually published, it's split into multiple structures.
Specifically, the data for each rollup is written independently, while a "base"
data type which contains all the other `SequencerBlock` info, plus the list of
rollup IDs in the block, is written. This allows each rollup to only require
the `SequencerNamespaceData` for the block and the `RollupNamespaceData` for
its own rollup transactions. For each block, if there are N rollup chain IDs
included, 1 + N structures are written to DA.

```rust
/// SubmittedMetadata represents the data written to the "base"
/// sequencer namespace. It contains all the other rollup IDs (and thus, 
/// namespaces) that were also written to in the same block.
pub struct SubmittedMetadata {
    /// The block hash obtained from hashing `.header`.
    block_hash: [u8; 32],
    /// The sequencer block header.
    header: SequencerBlockHeader,
    /// The rollup IDs for which `SubmittedRollupData`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1.RollupTransactions.id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    rollup_ids: Vec<RollupId>,
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.SequencerBlock.rollup_transactions_proof`.
    rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of
    /// the original sequencer block it was derived from.
    /// This proof together with `Sha256(MTH(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be 
    /// equal to `header.data_hash` which itself must match
    /// `astria.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.SequencerBlock.rollup_ids_proof`.
    rollup_ids_proof: merkle::Proof,
}
```

```rust
/// SubmittedRollupData represents the data written to a rollup namespace.
pub struct SubmittedRollupData {
    /// The hash of the sequencer block. Must be 32 bytes.
    sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencerblock.v1.RollupTransactions.rollup_id`
    rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    transactions: Vec<Bytes>,
    /// The proof that these rollup transactions are included in sequencer block.
    proof: merkle::Proof,
}
```

These structures contain all the information required for the reader of the
rollup data to verify that it is in fact what the sequencer chain finalized; ie.
the transactions are in the correct order, there are no transactions missing, or
transactions included that were not actually in the block. We can refer to these
properties as ordering, completeness, and correctness respectively. It is able
to do this *without* requiring the full transaction data of the block, as is
explained below.

## Data availability to rollup node

For a rollup node to verify the ordering, completeness, and correctness of the
block data it receives, it must verify the following:

1. the block was proposed by the correct proposer for that round
2. the block hash was in fact committed by the sequencer (ie. >2/3 stake voted
   to commit this block hash to the chain)
3. the block header correctly hashes to the block hash
4. the `data_hash` inside the header contains the `rollup_transactions_root` of the
   block (see [sequencer inclusion proofs](sequencer-inclusion-proofs.md) for
   details), which is a commitment to the `RollupDataSubmission`s in the block
5. the `transactions` inside `SubmittedRollupData` is contained within the
   `rollup_transactions_root`
6. the `rollup_ids_commitment` is a valid commitment to `rollup_ids`
7. the `data_hash` inside the header contains the `rollup_ids_commitment`
    for the block.

Let's go through these one-by-one.

Note: Tendermint validators will also validate all these fields before voting on
a block; thus, if a block is committed, we know the majority of validators
agreed that these fields are correct.

### 1. block proposer

The block header contains the proposer of the block. To verify the expected
proposer for a block, we obtain the validator set for that height, which
includes the proposer power for each validator. From this, we can calculate the
expected proposer for the height, and ensure it matches the proposer of the
block at that height.

### 2. block hash

Tendermint votes contain the block hash of the block the vote is for. Thus, when
verifying the votes for a block, we see what block hash was committed. The block
hash is a commitment to the entire block data.

To verify the commit for a block, we obtain the commit somehow (through a
sequencer node, or waiting for the next block which contains the commit for the
previous block). We also obtain the validator set for that height.

### 3. block header

The block hash is a commitment to the block header (specifically, the merkle
root of the tree where the leaves are each header field). We then verify that
the block header merkleizes to the block hash correctly.

### 4. `rollup_transactions_root`

The block's data (transactions) contain the `rollup_transactions_root` of the
block (see [sequencer inclusion proofs](sequencer-inclusion-proofs.md) for details),
which is a commitment to the `RollupDataSubmission`s in the block. Specifically,
the `rollup_transactions_root` is the root of a merkle tree where each leaf is a
 commitment to the rollup data for one spceific rollup. The block header contains
the field `data_hash` which is the merkle root of all the transactions in a block.
Since `rollup_transactions_root` is a transaction, we can prove its inclusion inside
`data_hash` (the `rollup_transactions_proof` field inside
`SubmittedMetadata`). Then, in the next step, we can verify that the rollup
data we received was included inside `rollup_transactions_root`.

### 5. `rollup_txs`

We calculate a commitment of the rollup data we receive (`rollup_txs` inside
`SubmittedRollupMetadata`). We then verify that this data is included inside
`rollup_transactions_root` (via the `proof` field inside
`SubmittedRollupMetadata`). At this point, we are now certain that the rollup data
we received, which is a subset of the entire block's data, was in fact committed
by the majority of the sequencer chain's validators.

### 6. `rollup_ids_root`

The `SubmittedMetadata` contains a list of the `rollup_ids` that were
included in the block. However, to ensure that rollup IDs are not omitted when
publishing the data (which would be undetectable to rollup nodes without forcing
them to pull the entire block's data), we also add a commitment to the rollup IDs
in the block inside the block's transaction data. We ensure that the
`rollup_ids` inside `SubmittedMetadata` match the
`rollup_ids_root`. This proves that no rollup IDs were omitted from the
published block, as if any were omitted, then the `rollup_ids_root` would
not match the commitment generated from `rollup_ids`.

### 7. `rollup_ids_root_inclusion_proof`

Similarly to verification of `rollup_transactions_root` inside `data_hash`, we also
verify an inclusion proof of `rollup_ids_root` inside `data_hash` when receiving
a published block.

## Exit point

Once all the verification steps have been completed, the rollup data is simply
passed to the rollup node's execution engine, which uses it to create the next
rollup block.
