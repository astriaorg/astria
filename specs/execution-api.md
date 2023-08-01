# Execution API Specification

## Overview

The Execution API is the interface `Conductor` uses to drive deterministic execution of sequenced blocks in dependent chains. Inspired by other APIs, such as the Engine API and ABCI Consensus API, it is a chain agnostic mechanism intended to be very simple to implement. It is a gRPC API which any state machine can implement, and use conductor with to drive their execution and integrate with the Astria Sequencer.

> Note: this documentation is for the v1alpha2 API which is currently being implemented, and some of the documentation is on how it should be implemented not how it is.

## Basic Design Principles

The Execution API is a resource based API with two resources: `Block` and `CommitmentState`. The API is designed to follow basic principles outlined by aip.dev as best practices for resource based APIs. gRPC has been chosen for the API due to the wide availability of language implementations which make it easy to generate client libraries, as well as stubs for server implementations.  

## Conductor Usage

### Startup

Upon startup, conductor needs to know the state of commitments in the state machine and calls `GetCommitmentState`. If started on a fresh rollup these will all be the same block. If running against a state machine with previous block data, Conductor must also track the block hash of any blocks between commitments, it will call `BatchGetBlock` to get block information between commitments.

### Execution & Commitments

From the perspective of the sequencer, `HEAD` commitments are made everytime the sequencer creates a new block at height N. The transactions on this block may change up until sequencer block at height N+1 has been created, at this point the block at height N has a `SOFT` commitment. A soft commitment means that sequencer consensus has full agreement on the block. Once a block has been seen on DA it has a `FIRM` commitment, it has been made widely available across the DA network.

When configuring conductor, you can configure the time at which blocks are executed in your rollup using the `execution_commitment_level` in the config file. If this is configured to a higher level of commitment, no action will be taken upon receiving lower commitments. 

`CreateBlock` is called to create a new execution chain block when the `execution_commitment_level` has been been reached for a given block. Upon receipt of a new block, the conudctor calls `UpdateCommitmentState` to update the commitment at the level of the `execution_commitment_level` and any level above it.

If `execution_commitment_level` is set to `HEAD`, `UpdateCommitmentState` will be called with an update to the `SOFT` commitment block after receiving a sequencer block with `prev_hash` which has already been executed on the chain.

If `execution_commitment_level` is set to `SOFT` or lower, `UpdateCommitmentState` will be called to update the `FIRM` commitment block when it's corresponding sequencer block has been seen in the DA

Note: For our EVM rollup, we map the `CommitmentState` to the `ForkchoiceRule`:
- `HEAD` Commitment -> `HEAD` Forkchoice
- `SOFT` Commitment -> `SAFE` Forkchoice
- `FIRM` Commitment -> `FINAL` Forkchoice

## Rollup Implementation Details

### CreateBlock

`CreateBlock` executes a set of given trasactions on the new chain. The following should be respected:

- `prev_block_hash` MUST match hash of the `SOFT` commitment state block, return `FAILED_PRECONDITION` otherwise.
- If block headers have timestamps, created block MUST have matching timestamp

### GetBlock

`GetBlock` returns information about a block given either it's `number` or `hash`. If the block cannot be found return a `NOT_FOUND` error.

### BatchGetBlocks

`BatchGetBlock` returns an array of Blocks which match the array of passed in block identifiers.

- The API endpoint MUST fail atomically, returning either all requested resource or a `NOT_FOUND` error.
- The returned objects MUST be in the same order as they were requested.

### GetCommitmentState

Returns the commitment state with execution `Block` information for each level of commitment.

### UpdateCommitmentState

`UpdateCommitmentState` replaces the 

- No commitment can ever decrease in block number on the blockchain, if this is attempted return a `FAILED_PRECONDITION` error.
- `HEAD` commitments MAY be to blocks which replace the previous `HEAD` with same block number.
- `SOFT` and `FIRM` block MUST either increase in block number OR match current commitment state block.
- `SOFT` and `FIRM` blocks MUST be members of the block chain starting from `HEAD`.
- Block numbers in state MUST be such that `SOFT` + 1 >= `HEAD` >= `SOFT` >= `FIRM`, return a `FAILED_PRECONDITION` error if this is not true

## Sequence Diagram

The sequence diagram below shows the API used within the full context of Astria stack. Demonstrating what happens between a user submitting a transactions, and seeing it executed as well as before soft and firm commitments.

![image](assets/execution_api_sequence.png)