# Astria Conductor

The Astria conductor connects the shared sequencer and DA layers to the execution layer. When a block is received from the sequencer layer or DA layer, the conductor filters out the transactions that are relevant to the rollup's namespace and pushes it to the execution layer. 

Blocks can be received via either:

 - The gossip network from the shared sequencer
 - The data availability layer, requested on a predefined interval

In the first case, the block is filtered and pushed to the execution layer, executed, and added to the blockchain. Brand new blocks are marked as head and all previous blocks are marked as a soft commitment. In general, the marking of blocks as safe will happen one block at a time as new blocks arrive. The block is not finalized until it's received from the data availability layer. 

In the second case, batches of blocks are received from the DA later, filtered for that rollup, and those blocks are used to set their corresponding blocks on the rollup as finalized.

The exact terminology that a rollup uses for its fork choice rules is up to its implementation. For example, Geth uses `head`, `safe`, and `final`. The conductor uses `head`, `soft`, and `firm`. The fork choice options are mapped with Geth in the following way:
 - `head` -> `head`
 - `soft` -> `safe`
 - `firm` -> `final`

The conductor has no knowledge that Geth uses the above options, it simply sends a `CommitmentState` message to the rollup, and the rollup does the rest.

## Architecture
The architecture of the conductor is inspired by the [Actor Model](https://en.wikipedia.org/wiki/Actor_model) with the actors within the conductor being the `Driver`, `Reader`, and `Executor`. Each actor operates concurrently and communicates with the other actors by passing messages. The conductor is written in Rust and utilized the Tokio runtime to achieve this.

![Conductor Architecture](assets/conductor-architecture.png)

The responsibilities of each module are as follows:
### Driver
 - Top level coordinator that runs and manages all the sub-components necessary for the Conductor
 - Creates the `Reader` and `Executor` actors on startup
 - Creates a gossip network for receiving data from the sequencer network
     - The gossip network uses the [astria-gossipnet](https://github.com/astriaorg/astria/tree/main/crates/astria-gossipnet)
 - Runs an event loop that handles receiving `DriverCommand`s and messages from the gossip network
 - Passes sequencer blocks received from the gossip network to the `Executor`

### Reader
 - Creates a `CelestiaClient` using the Celestia client implementation from `astria-sequencer-relayer` to communicating with the DA layer
 - Creates a `TendermintClient` which is used when validating blocks from the DA
   layer against the sequencer data
 - Runs an event loop that handles receiving `ReaderCommand`s that drives data retrieval from the DA layer
 - Passes the blocks it receives to the `Executor`

### Executor
 - Runs an event loop that handles receiving `ExecutorCommand`s from both the Driver and Reader
 - Filters out transactions by their rollup namespace and sends them to the rollup for execution (`soft` commits)
 - Catalogs and matches the block hashes received from the gossip network and the DA by rollup namespace to send `firm` commits to the rollup
 - Blocks are sent to the execution layer using [Astria’s GRPC Execution client interface](https://buf.build/astria/astria/docs/main:astria.execution.v1)
     - Rollups utilizing the Conductor must implement this interface

## Execution Data
### Transaction Filtering
The Conductor is designed so that for each rollup node running, an instance of the Conductor will be run with it.
The `chain_id` that Conductor uses for the rollup namespace is pulled from the rollup instance's config. As such, the Conductor is only aware of the single namespace of the rollup it is supporting. The `chain_id`/namespace is tracked in the sequencer block that contains the transactions for the rollup and not in the transactions themselves. When a sequencer block is received from the gossip network the Conductor only pulls out the transactions for its namespace and passes those transaction, and the previous execution hash from the rollup to the execution layer. See the [astria execution api](https://github.com/astriaorg/astria/blob/main/specs/execution-api.md) for more details.

### Soft Commitments
When single sequencer blocks are received by the Conductor from the gossip network, this data is treated as a `soft` commitment. Data received in this manner has been validated by the sequencer network and rollups can trust that the data will not be reverted. This is also the primary moment in the data life cycle for Astria that transactions are sent to the rollup for execution. As mentioned in the [Transaction Filtering](#transaction-filtering) section above, the only information sent to the rollup is the list of ordered transactions and the previous execution hash from the rollup. It is a given rollups responsibility to build their own specific block from the data provided and return the execution hash that adding the new block resulted in. The Conductor keeps a map of sequencer block hashes to rollup execution hashes for later matching when blocks are seen in the DA layer.

### Firm Commitments
When the Conductor pulls data from the DA, it again filters that data by namespace and compare the block hashes seen there with those of the already executed blocks stored in the map mentioned at the end of the Soft Commitments section. For each block seen in DA that matches an executed block, a `FinalizeBlock` message is sent to the rollup to set those blocks to `final` and the entries in the execution hash to sequencer block hash map are cleared.
If blocks are seen in DA data that haven't been seen via gossip, the transactions in those blocks are filtered for the namespace and sent to the rollup for execution as well as being set to final.
