# Astria Conductor

The Astria Conductor connects the shared sequencer and data availability layers to the execution layer, where the execution layer is a rollup execution environment. There is one instance of the Conductor per rollup node.

When a block is received, either from the Sequencer layer or from the DA layer, the Conductor filters for the transactions that are in the rollup's namespace and pushes them to the execution layer.

Blocks can be received via either:

- The gossip network from the shared Sequencer
- The data availability layer, requested on a predefined interval

In the first case, the transactions in the block are filtered and pushed to the execution layer, executed, and added to the blockchain. Brand new blocks are marked as head and all previous blocks are marked as a soft commitment. In general, the marking of blocks as safe will happen one block at a time as new blocks arrive. The block is not finalized until it's received from the data availability layer.

In the second case, batches of blocks are received from the DA layer and filtered for the rollup. These blocks are then used to set their corresponding blocks' commit status on the rollup as finalized.

The exact terminology that a rollup uses for its fork choice rules is up to its implementation. For example, Geth uses `head`, `safe`, and `final`. The Conductor uses `head`, `soft`, and `firm`. The fork choice options are mapped with Geth in the following way:

- `head` -> `head`
- `soft` -> `safe`
- `firm` -> `final`

To update the commitment level of a block on the rollup, it simply sends a `CommitmentState` message to the rollup node.

## Architecture

The architecture of the Conductor is inspired by the [Actor Model](https://en.wikipedia.org/wiki/Actor_model) with the actors within the Conductor being the `Driver`, `Reader`, and `Executor`. Each actor operates concurrently and communicates with the other actors by passing messages. The Conductor is written in Rust and utilizes the Tokio runtime to achieve this.

![Conductor Architecture](assets/conductor-architecture.png)

### Driver

- Top level coordinator that runs and manages all the subcomponents necessary for the Conductor
- Creates the `Reader` and `Executor` actors on startup
- Creates a p2p gossip network for receiving data from the Sequencer network
    - The gossip network uses the [astria-gossipnet](https://github.com/astriaorg/astria/tree/main/crates/astria-gossipnet)
- Runs an event loop that handles receiving `DriverCommand`s from other actors as well as messages from the gossip network
- Validates and passes Sequencer blocks received from the gossip network to the `Executor`

The Driver receives either `Events` ([link](https://github.com/astriaorg/astria/blob/6e71a76fa52c522ffdcabcd9d659e4de765d9d61/crates/astria-gossipnet/src/network_stream.rs#L39)) from the network, or `DriverCommand`s ([link](https://github.com/astriaorg/astria/blob/6e71a76fa52c522ffdcabcd9d659e4de765d9d61/crates/astria-conductor/src/driver.rs#L54)) from
the Conductor's internal event loop on a timer. The variants for `Event` and
`DriverCommand` that are relevant to the processing of blocks within the
Conductor are:

- `Event::GossipsubMessage(Message)` ([link](https://github.com/astriaorg/astria/blob/6e71a76fa52c522ffdcabcd9d659e4de765d9d61/crates/astria-gossipnet/src/network_stream.rs#L50))
    - The `Message` value within the `GossipsubMessage` contains the Sequencer
      block data from the Astria Sequencer. The `Message` is then parsed to a
      `SequencerBlockData`
      ([link](https://github.com/astriaorg/astria/blob/6e71a76fa52c522ffdcabcd9d659e4de765d9d61/crates/astria-sequencer-types/src/sequencer_block_data.rs#L39)).
      Once transformed, the block data is validated to make sure that the proposer
      for the block is the one expected. It also checks the commit of the parent
      block by verifying that >2/3 staking power of the sequencer chain voted for it.
      The block is then passed to the Executor actor as a
      `ExecutorCommand::BlockReceivedFromGossipNetwork` message which will
      ultimately process and filter the block.
- `DriverCommand::GetNewBlocks`([link](https://github.com/astriaorg/astria/blob/3c4e47dbe1818e4228691d6bfd2b2143a06f1a6e/crates/astria-conductor/src/driver.rs#L54))
    - This message triggers the sending of a `ReaderCommand::GetNewBlocks` to the
      Reader actor to initiate the pulling of data from the DA layer.

### Reader

- Creates a `CelestiaClient` to communicate with the DA layer
- Creates a `TendermintClient` which is used when validating blocks received
- Runs an event loop that handles receiving `ReaderCommand`s that drive data retrieval from the DA layer
- Passes the blocks it receives to the `Executor`

The Reader receives a `ReaderCommand::GetNewBlocks`
([link](https://github.com/astriaorg/astria/blob/3c4e47dbe1818e4228691d6bfd2b2143a06f1a6e/crates/astria-conductor/src/driver.rs#L54))
message from the driver. The `CelestiaClient`
([link](https://github.com/astriaorg/astria/blob/3c4e47dbe1818e4228691d6bfd2b2143a06f1a6e/crates/astria-sequencer-relayer/src/data_availability.rs#L244))
is then called from the Reader to get data from the Celestia DA. This data is
then parsed from Celestia blobs into individual partial blocks (consisting of
relevant information needed for validation + the relevant rollup transactions).
The block data is then validated to make sure that the proposer for the block
is the one expected. It also checks the commit of the parent block by verifying
that >2/3 staking power of the sequencer chain voted for it.
Each block is then transformed into a `SequencerBlockSubset`s and handed off to the Executor along
with the command `ExecutorCommand::BlockReceivedFromDataAvailability`, then it is sent to the rollup
for execution.

### Executor

- Runs an event loop that handles receiving `ExecutorCommand`s from both the
  Driver and Reader
- Filters transactions by their rollup namespace and sends them to the rollup for execution
- Maps sequencer block hashes to execution block hashes so that it can send `firm` commits to the rollup
- Blocks are sent to the execution layer using [Astria’s GRPC Execution client interface](https://buf.build/astria/astria/docs/main:astria.execution.v1alpha1)
    - Rollups utilizing the Conductor must implement this interface
- If a block comes from the DA layer, a "finalize block" message is sent to the rollup

The `ExecutorCommand` ([link](https://github.com/astriaorg/astria/blob/eeffd2dc24ec14cbc7a3b3197ec2a3c099a78605/crates/astria-conductor/src/executor.rs#L81)) variants that the Executor receives are as follows:

- `ExecutorCommand::BlockReceivedFromGossipNetwork` commands are received when data comes from the Sequencer.
- `ExecutorCommand::BlockReceivedFromDataAvailability` commands are received
  when data comes from the DA layer.

When blocks are received from the gossip network, their transactions are
filtered based on the rollup's namespace, then are sent to the rollup for
execution. The execution hash that is returned from the rollup is then stored in
a hash map for Sequencer block hash -> execution hash.

When blocks are received from the DA layer, the hash map of Sequencer block hash
-> execution hash is checked to see if the block has already passed through the
Conductor from the Sequencer. If the block isn't seen, it is filtered and sent
to the rollup for execution exactly the same way the transactions are sent when
received from the Sequencer, then a message to finalize the block is sent.
If the block is already present in the hash map, just the finalize block message
is sent. After being finalized, the Sequencer block hash -> execution hash entry
in the hash map is deleted.

#### The Execution Queue
The Queue within the Executor is responsible for verifying the ordering of
blocks received only from the Sequencer. The basic flow for the blocks into 
and out of the Queue, is as follows:


1. Blocks are received from the Sequencer and are validated in the Driver.
2. Validated blocks are then sent to the Executor and ultimately to `execute_block`
3. Once inside `execute_block` that block is added to the Queue.
4. The Queue verifies the order of the blocks to make sure that they are
   following the CometBFT fork choice rules.
5. All blocks that can be executed are then popped from the queue
   and are individually passed to the rollup for execution.

The different fork choice rules are set using the `execution_commit_level`.
There are three options:

1. `HEAD`: The HEAD setting means that every time the sequencer creates a new
   block at head height N, that block will get sent to the execution layer. The
   HEAD blocks at height N can be reorged or updated until
   a block at height N+1 has been received, so multiple blocks at the head
   height can be sent to execution. Once the N+1 block is received, its
   parent block at height N is set to `SOFT` and N+1 becomes the new HEAD
   height. When blocks are seen in DA, they are then set to FIRM.
2. `SOFT`: The SOFT setting means that only blocks that have full sequencer
   consensus agreement will be sent to the execution layer and will not be
   reorged. Internally this means that the queue will hold all blocks at the
   head height, but only return blocks that have a child. Thus all blocks sent
   to the execution layer are always soft. Blocks are marked as FIRM when they are seen in DA. 
3. `FIRM`: The FIRM setting indicates that only blocks that have been written
   and propagated across the DA network will be sent to the execution layer. All
   blocks sent will always be FIRM.

## Execution Data

### Transaction Filtering

An instance of the Conductor is meant to be run alongside the rollup node.

The `chain_id` that Conductor uses as the rollup's identifier is pulled from the rollup's config.
When a user submits a transaction to be sequenced, they specify the `chain_id` of its destination.

When a Sequencer block is received from the gossip network, the Conductor filters the transactions
for its chain ID and executes only those transactions on top of its parent state. See the
[astria execution api](https://github.com/astriaorg/astria/blob/main/specs/execution-api.md)
for more details.

### Data Validation

Data is validated before being sent to the rollup for execution. Validation occurs in two places:

- When blocks are received from the gossip network, the data is validated in Driver's `handle_network_event` using `BlockVerifier::validate_sequencer_block_data`
- When blocks are fetched from the DA layer, the data is validated in Reader's `get_new_blocks` using `BlockVerifier::validate_signed_namespace_data` and `BlockVerifier::validate_rollup_data`

### Soft Commitments

When a Sequencer block is received by the Conductor from the gossip network, it is
assumed to be the `head` of the chain, thus its parent block is designated as a
`soft` commitment. This is also the primary moment in the data life cycle for Astria
that transactions are sent to the rollup for execution.

As mentioned in the [Transaction Filtering](#transaction-filtering) section above, the only information sent to the rollup is the list of ordered transactions and the previous execution hash from the rollup. It is the rollup node's responsibility to build their own specific block from the data provided and return the execution hash that resulted from adding the new block.

The Conductor keeps a map of Sequencer block hashes to rollup execution hashes for later matching when blocks are seen in the DA layer.

### Firm Commitments

When the Conductor pulls data from the DA, it compares the Sequencer block hashes
seen with those of the already executed blocks stored in the map mentioned at the
end of the [Soft Commitments](#soft-commitments) section. For each block seen in
DA that matches an executed block, a `FinalizeBlock` message is sent to the rollup
to set those blocks to `final` and the entries in the execution hash to Sequencer
block hash map are cleared.

If blocks are seen in DA data that haven't been seen via gossip, the transactions in those blocks are filtered for the namespace and sent to the rollup for execution as well as being set to final.
