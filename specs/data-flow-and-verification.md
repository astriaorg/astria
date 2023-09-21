# Data flow and verification

This document addresses how rollup data flows throughout the system and is verified throughout the system.

## Background

The purpose of the Astria sequencer is to batch, order, and commit to data on behalf of rollups. The entry point of data is transactions sent by users, and the exit point is execution by a rollup node.

## Entry point

The entry point for rollup data is via a `sequence::Action`, which can become part of a sequencer transaction. The data types are as follows:

```rust
// sequence::Action
pub struct Action {
    pub(crate) chain_id: Vec<u8>,
    pub(crate) data: Vec<u8>,
}
```

```rust
// an unsigned transaction
pub struct Unsigned {
    pub(crate) nonce: Nonce,
    pub(crate) actions: Vec<Action>,
}
```

The `data` field inside the `sequence::Action` is arbitrary bytes, which should be an encoded rollup transaction. The sequencer is agnostic to the transaction format of the rollups using it. The `chain_id` field is an identifier for the rollup the data is destined for. 

To submit rollup data to the system, the user creates a transaction with a `sequence::Action` within it and signs and submits it to the sequencer. The sequencer will then include it in a block, thus finalizing its ordering.

## Sequencer to data availability

Once a transaction (and the actions within it) is included in a sequencer block, the block data is published via a data availability layer.

The block data published is as follows:
```rust
pub struct SequencerBlockData {
    block_hash: Hash,
    header: Header,
    /// This field should be set for every block with height > 1.
    last_commit: Option<Commit>,
    /// chain ID -> rollup transactions
    rollup_data: BTreeMap<ChainId, Vec<Vec<u8>>>,
    /// The root of the action tree for this block.
    action_tree_root: [u8; 32],
    /// The inclusion proof that the action tree root is included
    /// in `Header::data_hash`.
    action_tree_root_inclusion_proof: InclusionProof,
    /// The commitment to the chain IDs of the rollup data.
    /// The merkle root of the tree where the leaves are the chain IDs.
    chain_ids_commitment: [u8; 32],
}
```

When this data is actually published, it's split into multiple structures. Specifically, the data for each rollup is written independently, while a "base" data type which contains the rollup chain IDs  included in the block is also written. This allows each rollup to only require the `SequencerNamespaceData` for the block and the `RollupNamespaceData` for its own rollup transactions. For each block, if there are N rollup chain IDs included, 1 + N structures are written to DA.


```rust
/// SequencerNamespaceData represents the data written to the "base"
/// sequencer namespace. It contains all the other chain IDs (and thus, namespaces) that were
/// also written to in the same block.
#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub last_commit: Option<Commit>,
    pub rollup_chain_ids: Vec<ChainId>,
    pub action_tree_root: [u8; 32],
    pub action_tree_root_inclusion_proof: InclusionProof,
    pub chain_ids_commitment: [u8; 32],
}
```

```rust
/// RollupNamespaceData represents the data written to a rollup namespace.
#[derive(Serialize, Deserialize, Debug)]
pub struct RollupNamespaceData {
    pub(crate) block_hash: Hash,
    pub(crate) chain_id: ChainId,
    pub rollup_txs: Vec<Vec<u8>>,
    pub(crate) inclusion_proof: InclusionProof,
}
```

These structures contain all the information required for the reader of the rollup data to verify that it is in fact what the sequencer chain finalized; ie. the transactions are in the correct order, there are no transactions missing, or transactions included that were not actually in the block. We can refer to these properties as ordering, completeness, and correctness respectively.

Note that the `Header` field in `SequencerNamespaceData` is a [Tendermint header](https://github.com/informalsystems/tendermint-rs/blob/4d81b67c28510db7d2d99ed62ebfa9fdf0e02141/tendermint/src/block/header.rs#L25).

## Data availability to rollup node

For a rollup node to verify the ordering, completeness, and correctness of the block data it receives, it must verify the following:

1. the block was proposed by the correct proposer for that round
2. the block hash was in fact committed by the sequencer (ie. >2/3 stake voted to commit this block hash to the chain)
3. the block header correctly hashes to the block hash
4. the `data_hash` inside the header contains the `action_tree_root` of the block (see [sequencer inclusion proofs](sequencer-inclusion-proofs.md) for details), which is a commitment to the `sequence:Action`s in the block
5. the `rollup_txs` inside `RollupNamespaceData` is contained within the `action_tree_root`
6. the `chain_ids_commitment` is a valid commitment to `rollup_chain_ids`
7. `last_commit` is a valid Tendermint commit to the parent block hash (TODO: I think these can just be changed to the block's commit at time of writing, even if it's not canonical)
8. `last_commit_hash` inside the header is a valid commitment to the `last_commit`

Let's go through these one-by-one. 

#### 1. block proposer

The block header contains the proposer of the block. To verify the expected proposer for a block, we obtain the validator set for that height, which includes the proposer power for each validator. From this, we can calculate the expected proposer for the height, and ensure it matches the proposer of the block at that height.

#### 2. block hash

Tendermint votes contain the block hash of the block the vote is for. Thus, when verifying the votes for a block, we see what block hash was committed. The block hash is a commitment to the entire block data.

To verify the commit for a block, we obtain the commit somehow (through a sequencer node, or waiting for the next block which contains the commit for the previous block). We also obtain the validator set for that height. 

#### 3. block header

#### 4. `action_tree_root`

#### 5. `rollup_txs`

#### 6. `chain_ids_commitment`

## Exit point

Once all the verification steps have been completed, the rollup data is simply passed to the rollup node's execution engine, which uses it to create the next rollup block.
