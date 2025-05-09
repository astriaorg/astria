# Execution API (v2) Specification

## Overview

The Execution API is the interface `Conductor` uses to drive deterministic derivation
of a rollup chain from Sequencer and Celestia blocks. Inspired by other APIs, such
as the Engine API and ABCI Consensus API, it is a chain-agnostic mechanism intended
to be very simple to implement. It is a gRPC API which any state machine can implement
and use Conductor with to drive their block creation to integrate with the Astria
Sequencer.

## Basic Design Principles

The Execution API is a resource-based API with three resources: `ExecutedBlockMetadata`,
`CommitmentState`, and `ExecutionSession`. The API is designed to follow basic
principles outlined by aip.dev as best practices for resource based APIs. gRPC
has been chosen for the API due to the wide availability of language implementations
which make it easy to generate client libraries and server implementations.

## Conductor Usage

### Execution Sessions

Conductor driven execution of blocks occurs in execution sessions, which contain
a bound of block heights/numbers to be executed. Every session contains a lower
bound (start block number) and may also contain an upper bound (end block number).
RPCs within the current session are verified by the server-side application via
the `session_id` in `GetExecutedBlockMetadataRequest` and `ExecuteBlockRequest`.

If an upper bound to the current execution session is specified in the `ExecutionSessionParameters`,
Conductor will stop executing past the end block number and request
a new session via `CreateExecutionSession`. This allows for providing Conductor
with new session parameters (such as chain ID, height bounds, etc.) and can be
used to perform rolling hard forks.

### Startup

Upon startup, conductor starts its first execution session by calling `CreateExecutionSession`.
This returns an `ExecutionSession`, containing the necessary information for the
client to drive execution for the duration of the session.

### Execution & Commitments

From the perspective of the Conductor:

- `Soft` commitments have been fully committed by the Sequencer consensus.
- A `Firm` commitment indicates that the block has been written and propagated
  across Celestia.

When configuring Conductor, the threshold at which blocks are executed on the rollup
can be set via the `execution_commitment_level` in the config file. `ExecuteBlock`
is called to create a new rollup block when the `execution_commitment_level` has
been reached for a given block. Upon receipt of the executed block, Conductor calls
`UpdateCommitmentState` to update the commitment at the level of the
`execution_commitment_level` and any level above it.

`execution_commitment_level` options and changes to execution:

- `SoftOnly`
  - upon receiving a new sequencer block N from the Sequencer:
    - `ExecuteBlock` will be called with data from the Sequencer block N, then
    - `UpdateCommitmentState` will be called to update the `soft` block to N
- `FirmOnly`
  - conductor does not need to listen for new blocks from Sequencer
  - upon reading a new Sequencer block N from Celestia:
    - `ExecuteBlock` will be called with data from the Sequencer block N
    - `UpdateCommitmentState` will be called to update `firm` and `soft` blocks
      to N
- `SoftAndFirm`
  - **NOTE:** the regular operation of Conductor is such that "soft" blocks from
    Sequencer are received prior to their "firm" Celestia counterparts, but Conductor
    is capable of handling the reverse scenario as well due to the logic below.
  - upon receiving a new Sequencer block N from the Sequencer:
    - if Sequencer block `N` has not yet been read from Celestia (this is considered
      normal operation):
      - `ExecuteBlock` will be called with data from the Sequencer block N, then
      - `UpdateCommitmentState` will be called to update the `soft` block to N
    - if Sequencer block `N` has already been read from Celestia:
      - Block is skipped
  - upon reading Sequencer block N from Celestia:
    - if Sequencer block N has already been read from the Sequencer (this is expected
      to be regular operation):
      - `UpdateCommitmentState` will be called to update the `firm` block to N
    - if Sequencer block N has not yet been read from Sequencer:
      - `ExecuteBlock` will be called with data from the Sequencer block N, then
      - `UpdateCommitmentState` will be called to update the `firm` and `soft`
        blocks to N

Note: For our EVM rollup, we map the `CommitmentState` to the `ForkchoiceRule`:

- `Soft` Commitment -> `HEAD` Forkchoice && `SAFE` Forkchoice
- `Firm` Commitment -> `FINAL` Forkchoice

### Celestia Search Height

`CommitmentState` contains the field `lowest_celestia_search_height`, representing
the lowest Celestia height that will be searched for the next firm block. In the
current implementation of Conductor, this is set to the Celestia height at which
the most recent firm block was found.

There are, however, many factors that can result in the same Sequencer blocks sharing
or skipping Celestia heights. As such, the Celestia heights correlating to Sequencer
commitments may not increase linearly, and a range of heights must be searched.
Conductor begins fetching at `CommitmentState.lowest_celestia_search_height`, continuing
searching for firm commitments until it reaches `lowest_celestia_search_height +
ExecutionSessionParameters.celestia_search_height_max_look_ahead`. This also allows
for Conductor to be resilient against any potentially missing Sequencer blocks
on Celestia or Sequencer blocks which end up out of order on Celestia.

## Rollup Implementation Details

### CreateExecutionSession

`CreateExecutionSession` returns an `ExecutionSession`, defining all necessary information
for the client to begin driving execution. This includes `ExecutionSessionParameters`,
containing the necessary information for network connection and mapping rollup blocks
to Astria Sequencer (soft) and Celestia (firm) blocks. Also returned is the current
`CommitmentState` of the server and a `session_id` to be supplied in RPC requests
for the current session. The API is agnostic as to how this information is defined
in a rollup's genesis, and is only used by the Conductor as configuration at the
beginning of each execution session.

**Note**: calls to `CreateExecutionSession` **MUST** invalidate any previous execution
sessions.

### ExecuteBlock

`ExecuteBlock` executes a set of given transactions on top of the chain
indicated by `parent_hash`. The following should be respected:

- `parent_hash` **MUST** match hash of the most recently committed block, whether
  `soft` or `firm`. RPC **MUST** return `FAILED_PRECONDITION` status otherwise.
- If block headers have timestamps, the created block MUST have matching timestamp
- **Note:** The `CommitmentState` is NOT modified by the execution of the block.
- It is up to the execution node if it includes the `sequencer_block_hash`
  provided as a part of the executed block metadata. If utilized, the server **MUST**
  throw an `INVALID_ARGUMENT` error if the `sequencer_block_hash` is not included
  in the request.
- `ExecuteBlock` **MUST** return `PERMISSION_DENIED` status if the provided execution
  session ID is incorrect, the current session is not valid, or a current execution
  session does not exist.
- `ExecuteBlock` **MUST** return `OUT_OF_RANGE` if the block is out of the bounds
  of the current execution session.

### GetExecutedBlockMetadata

`GetExecutedBlockMetadata` returns information about a block given its `ExecutedBlockIdentifier`,
consisting of either a `hash` or `number` (as determined by the server, *not* by
the Sequencer or the Celestia height). If the block cannot be found, return a `NOT_FOUND`
error.

### UpdateCommitmentState

`UpdateCommitmentState` replaces the current `CommitmentState` on the rollup.

- No `UpdateCommitmentStateRequest` can ever decrease in either `soft` or `firm`
  block number.
- `soft` blocks **MUST** increase in block number each time this is called.
- `firm` blocks **MUST** either increase in block number ***or*** match the current
  commitment state block.
- Block numbers in state **MUST** be such that  `soft` >= `firm`.
- If any of these conditions fail, a `FAILED_PRECONDITION` status **MUST** be returned.
- `UpdateCommitmentState` **MUST** return `PERMISSION_DENIED` status if the provided
  execution session ID is incorrect, the current session is not valid, or a current
  execution session does not exist.
- `UpdateCommitmentState` **MUST** return `OUT_OF_RANGE` if the provided state's
  soft height is out of bounds of the current session.
- `UpdateCommitmentState` **MUST** return `OUT_OF_RANGE` if the provided state's
  firm height is changed from the current state and is out of bounds of the current
  session. If the firm height is equal to that of the current commitment state,
  it should not be checked, as it may not have been updated since the previous
  execution session.

## Sequence Diagram

The sequence diagram below shows the API used within the full context of Astria
stack, demonstrating what happens between when a user submits a transaction and
when they see it executed, as well as the process of soft and firm commitments.
Note that this diagram presumes the Conductor is running in `SoftAndFirm` mode,
driving execution when it receives a soft block. If it were running in `FirmOnly`
mode, `ExecuteBlock` would not be called until the block is received from the
Data Availability layer.

![image](assets/execution_api_sequence.png)
