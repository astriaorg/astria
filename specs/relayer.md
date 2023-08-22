# Astria Sequencer Relayer

The Astria Sequencer Relayer (referred to as relayer from here on out), reads
new blocks from the sequencer, pushes them to the gossipnet which Conductor
listens on, and writes batches sets of blocks to DA.

It is run as a sidecar to a sequencer proposer, and each instance only
relays/writes data from it's proposer.

## Interfaces

### Sequencer

Relayer fetches new blocks from the CometBFT consensus node of the sequencer
using [CometBFT's Block RPC](https://docs.cometbft.com/v0.37/spec/rpc/#block)
with the `height` set to null to grab the latest block. This returns the latest
proposed block, which is not fully finalized by the sequencer network yet.

### P2P/Gossipnet

> Note: The P2P/gossipnet may be removed/replaced by an alternate model in the
> future. It's known that this is heavy weight and is propogating data to all
> rollups which they are not all interested in.

After receiving a block from the sequencer, if it is signed by the relayers
proposer, it is converted into the `SequencerBlockData` shape
([link](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-types/src/sequencer_block_data.rs#L39-L46)).
This object is then pushed to the libp2p "gossipnet" network, for execution by
rollups. This contains information for execution by all rollups. 

### Data Availability

Relayer also writes data to Celestia DA. Since Astria block times can be much
faster than Celestia block times, we create a queue of yet to be written blocks.
`SequencerBlockData` is added to a queue for DA write after being pushed to the
P2P layer.

A collection of blobs is written in one transaction to DA. For each Astria block
there are 1 + N blobs written to Celestia, where N is the number of rollups who
have transactions in the Astria block. All of these are serialized to JSON and
then to bytes and signed with the sequencer proposer key. The format of these is
as follows:

- Sequencer block information, 1 per Astria block, of type
  `SequencerNamespaceData`
  ([link](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-relayer/src/data_availability.rs#L138))
  serialized to 
- Rollup transaction data, N per Astria block, of type `RollupNamespaceData`
  ([link](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-relayer/src/data_availability.rs#L149))

The sequencer block information is written to the [Astria
Namespace](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-types/src/namespace.rs#L21),
and rollup transaction data is written to a namespace which is deterministically
derived from the `chainId` for the rollup tx
([link](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-types/src/namespace.rs#L44)).

These blobs are submitted to Celestia via the `State.SubmitPayForBlob` JSON RPC
([State APIs](https://node-rpc-docs.celestia.org/#state)). This job is blocking
until it is included, once it has been included the next set of queued
information is grabbed to be pushed.
