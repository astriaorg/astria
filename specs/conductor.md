# Astria Conductor

The Astria conductor connects the shared sequencer and DA layers to the execution layer. When a block is received from the sequencer layer or DA layer, the conductor filters out the transactions that are relevant to the rollup's namespace and pushes it to the execution layer. 

There are two ways for a block to be received:

 - via the gossip network from the shared sequencer
 - via the data availability layer, requested on a predefined interval

In the first case, the block is filtered and pushed to the execution layer, executed, and added to the blockchain. Brand new blocks are marked as head and all previous blocks are marked as a soft commitment. The block is not finalized until it's received from the data availability layer. 

In the second case, batches of blocks are received from the DA later and those blocks are marked as finalized in the given rollup.

## Architecture
The architecture of the conductor is inspired by the [Actor Model](https://en.wikipedia.org/wiki/Actor_model) with the actor modules within the conductor being the `Driver`, `Reader`, and `Executor`.

![Conductor Architecture](assets/conductor-architecture.png)

The responsibilities of each module are as follows:
### Driver
 - Top level coordinator that runs and manages all the sub-components necessary for this application.
 - Creates the `Reader` and `Executor` actors on startup
 - Creates a gossip network for receiving data from the sequencer network
 - Runs an event loop that handles receiving `DriverCommand`s and messages from the gossip network.

### Reader
 - Reuses `CelestiaClient` from `astria-sequencer-relayer` to get data from the data availability layer. It then sends the blocks to the `Executor`
 - Creates a `CelestiaClient` used for communicating with the data availability layer
 - Creates a `TendermintClient` which is used when validating blocks from the DA layer
 - Runs an event loop that handles receiving `ReaderCommand`s that drives the logic described in the above items

### Executor
 - Uses [Astria’s GRPC Execution client](https://buf.build/astria/astria/docs/main:astria.execution.v1) to send blocks to the execution layer
     - This requires the execution client to run the [Astria GRPC ExecutionService](https://buf.build/astria/astria/docs/main:astria.execution.v1)
 - Runs an event loop that handles receiving `ExecutorCommand`s from both the driver and reader
 - 
