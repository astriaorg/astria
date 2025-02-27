# Astria Conductor

## Overview

Astria's *Conductor* executes transactions sequenced by Astria's *Sequencer*
against a rollup (currently geth). It does this by:

1. reading data specific to the rollup from Sequencer or from a data
   availability provider (currently Celestia);
2. and then executing that data against the rollup implementing the
   [`astria.execution.v2` API](./execution-api.md).

Executed rollup data that is read directly from Sequencer is referred to
*soft*-committed, while rollup data read from the data availability provder
is referred to a *firm*-committed.

Conductor is intended to be a side-car to a rollup node.

## Application logic

Conductor can be run in *soft-only*, *firm-only*, and *soft-and-firm* modes
and are explained below.

### Soft-only mode

In soft-only mode, Conductor only reads rollup information from Sequencer but
not the data availability provider. It connects to a
**fully trusted Sequencer node**.

At a high level, it followed the following steps (all remote procedure calls
are gRPC):

1. Call `astria.execution.v2.CreateExecutionSession` to initiate a new execution
  session (call this `E`).
2. Map the current rollup's soft number/height to the next expected Sequencer's
  height using `S = E.sequencer_start_block_height + (C.soft.number - E.rollup_start_block_number`.
3. Call `astria.sequencerblock.v1.GetFilteredSequencerBlock` with
  arguments `S` and `E.rollup_id` to get Sequencer block metadata and data
  specific to Conductor's rollup node.
4. Call `astria.execution.v2.ExecuteBlock` with the result of step 3.
5. Call `astria.execution.v2.UpdateCommitmentState` with the result of
  step 4, specifically updating the tracked commitment state
  `C.soft.number += 1`.
6. Go to step 2.

### Firm-only mode

In firm-only mode, Conductor only reads rollup information from Celestia but
not from Sequencer. Because Sequencer blocks are both batched and split by
namespaces (see the [Sequencer-Relayer spec](./sequencer-relayer.md)),
Conductor must read, verify, and match Sequencer block metadata to rollup data
for a given Sequencer height.

At a high level, it followed the following steps (all remote procedure calls
are gRPC):

1. Call `astria.execution.v2.CreateExecutionSession` to initiate a new execution
  session (call this `E`).
2. Call Sequencer's CometBFT JSONRPC endpoint with arguments
  `{ "method": "genesis", "params": null }` to get its genesis
  state (call this `G`).
3. Determine the rollup's [Celestia v0 namespace] from the first 10 bytes of its
  ID, `G.rollup_id[0..10]` (call this Celestia namespace `Nr`)
4. Determine the Sequencer's [Celestia v0 namespace] from the first 10 bytes of
  the Sha256 hash of its chain ID, `Sha256(Gs.chain_id)[0..10]` (call this
  Celestia namespace `Ns`).
5. Map the current rollup's firm number/height to the Sequencer's height using
  `F = E.sequencer_start_block_height + (C.firm.number - E.rollup_start_block_number`.
6. Determine the permissible Celestia height window that Conductor is allowed
  to read from `H_start = C.lowest_celestia_search_height` and
  `H_end = H_start + E.celestia_search_height_max_look_ahead * 6`[^1].
7. For every height `H` in the range `[H_start, H_end]` (inclusive):
    1. Call Celestia-Node JSONRPC with arguments to get Sequencer block metadata
      `{"method": "blob.GetAll", "params": [<H>, [<Ns>]]}`.
    2. Decompress the result of 1. as brotli, decode as protobuf
      `astria.sequencerblock.v1.SubmittedMetadataList`.
    3. For each metadata element found in the previous step:
        1. Call the Sequencer CometBFT JSONRPC with the following arguments to
          get the commitment at the metadata sequencer height `M`
          `{"method": "commit", "params": { "height": <M>}}`.
        2. Call Sequencer CometBFT JSONRPC with the following arguments to
          get the set of validators at the metadata sequencer height `M-1`
          (the validators for height `M` are found at height `M-1`):
          `{"method": "validators", "params": { "height": <M-1>}}`.
        3. Validate the metadata using the commitment and validator's
          information.
    4. Call Celestia-Node JSONRPC with arguments to get Rollup data
      `{"method": "blob.GetAll", "params": [<H>, [<Nr>]]}`.
    5. Decompress the result of 4. as brotli, decode as protobuf
      `astria.sequencerblock.v1.SubmittedRollupDataList`.
    6. Match pairs `P = (metadata, rollup_data)` found in the previous steps
      using `rollup_data.block_hash` and `metadata.block_hash`.
8. Get that pair `P` with metadata sequencer height matching the next expected
  firm Sequencer height `M == F` (as determined in step 5). If it exists, go to
  step 9. If no such pair exists, exit.
9. Call `astria.execution.v2.ExecuteBlock` with the result of step 9.
10. Call `astria.execution.v2.UpdateCommitmentState` with the result of
  step 9, specifically updating the tracked commitment state
  `C.firm.number == C.soft.number += 1`[^2] and `C.lowest_celestia_search_height
   = H`,
  with `H` the source Celestia height of the just executed pair `P`.
11. Go to step 5.

[Celestia v0 namespace]: https://celestiaorg.github.io/celestia-app/specs/namespace.html#version-0
[^1]: It is assumed that on average 6 Sequencer heights will fit into 1
  Celestia height due to the default Sequencer block time being 2s and
  Celestia being 12s.
[^2]: In firm-only mode the soft and firm commitments are updated in lock-step
  because soft commitments must not trail firm by contract.

### Soft-and-firm mode

Soft-and-firm mode operates as the union of soft-only and firm-only modes,
running independent tasks that perform exactly the same steps, with the
exception of the execution and update-commitment steps:

If the soft commitment is ahead of firm,
`CommitmentState.soft.number > CommitmentState.firm.number`, then step
`firm-only.9` is skipped (i.e. the data is not executed against the rollup),
but only step `firm-only.10` is ran *without updating the soft number (i.e.
only `CommitmentState.firm.number += 1` is advanced).

Soft being ahead of firm is the expected operation. In certain rare situations
the numbers can match exactly, and step `firm-only.9` and `firm-only.10` are
executed as written.

## Startup, Restarts, Execution, and Commitments

See [`astria.execution.v2` API documentation](./execution-api.md) for more
information on Conductor startup, restart, execution, and commitment logic.
