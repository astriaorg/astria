# Astria Sequencer Relayer

The Astria Sequencer Relayer (Relayer), is a stateless service which reads new
blocks from the sequencer, pushes them to the gossipnet for rollup execution,
and writes batches sets of blocks to DA.

It is run as a sidecar to a sequencer proposer, and each instance only
relays/writes data from it's proposer.

## Interfaces

### API

Relayer offers basic status/health check endpoints via an HTTP server with the
following endpoints:

- `readyz`
  - path: `/readyz`
  - input: none
  - output either:
    - status: 200, `ok`
    - status: 503, `not ready`
- `healthz`
  - path: `/healthz`
  - input: none
  - output either:
    - status: 200, `ok`
    - status: 504, `degraded`
- `status`
  - path: `/status`
  - input: none
  - output:
    - json:
      - `data_availability_connected`
        - boolean status of DA connection,
      - `sequencer_connected`
        - boolean status of sequencer connection
      - `current_sequencer_height`
        - integer height of last seen sequencer block
      - `current_data_availability_height`
        - integer height of last known DA block height
      - `number_of_subscribed_peers`
        - integer number of subscribed gossipnet members

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

```rust
/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
pub struct SequencerBlockData {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    /// This field should be set for every block with height > 1.
    pub(crate) last_commit: Option<Commit>,
    /// namespace -> rollup txs
    pub(crate) rollup_txs: HashMap<Namespace, Vec<Vec<u8>>>,
}
```

### Data Availability

Relayer also writes data to Celestia DA. Since Astria block times can be much
faster than Celestia block times, we create a queue of yet to be written blocks.
`SequencerBlockData` is added to a queue for DA write after being pushed to the
P2P layer.

A collection of blobs is written in one transaction to DA. For each Astria block
there are 1 + N blobs written to Celestia, where N is the number of rollups who
have transactions in the Astria block. The types of these data blobs are defined
as such:

- Sequencer block information, 1 per Astria block, of type
  `SequencerNamespaceData`
  ([link](https://github.com/astriaorg/astria/blob/main/crates/astria-sequencer-relayer/src/data_availability.rs#L147))

  ```rust
  /// SequencerNamespaceData represents the data written to the "base"
  /// sequencer namespace. It contains all the other namespaces that were
  /// also written to in the same block.
  pub struct SequencerNamespaceData {
      pub block_hash: Hash,
      pub header: Header,
      pub last_commit: Option<Commit>,
      pub rollup_namespaces: Vec<Namespace>,
  }
  ```

- Rollup transaction data, N per Astria block, of type `RollupNamespaceData`
  ([link](https://github.com/astriaorg/astria/blob/main/crates/astria-sequencer-relayer/src/data_availability.rs#L158))

  ```rust
  /// RollupNamespaceData represents the data written to a rollup namespace.
  pub struct RollupNamespaceData {
      pub(crate) block_hash: Hash,
      pub(crate) chain_id: Vec<u8>,
      pub rollup_txs: Vec<Vec<u8>>,
      pub(crate) inclusion_proof: InclusionProof,
  }
  ```

Each of these entries is then signed by the proposer key and wrapped as
`SignedNamespacedData` (link), which will is parsed into bytes for inclusion in
a Celestia Blob.

```rust
pub struct SignedNamespaceData<D> {
    pub data: D,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

pub struct Blob {
    pub namespace_id: [u8; NAMESPACE_ID_AVAILABLE_LEN],
    pub data: Vec<u8>,
}
```

The sequencer block information is written to the [Astria
Namespace](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-types/src/namespace.rs#L21),
and rollup transaction data is written to a namespace which is deterministically
derived from the `chainId` for the rollup tx
([link](https://github.com/astriaorg/astria/blob/7ebb743ed6f1d9eef69372f2cbb4ab9cbe2668b3/crates/astria-sequencer-types/src/namespace.rs#L44)).

These blobs are submitted to Celestia via the `State.SubmitPayForBlob` JSON RPC
([State APIs](https://node-rpc-docs.celestia.org/#state)). This job is blocking
until it is included, once it has been included the next set of queued
information is grabbed to be pushed.

## Event Loops

```text
            ┌───────────────────────┐
            │ Sequencer/Gossip Loop │
            └───────────────────────┘
                        │
                        │
                        │
                        ▼
  ┌──────────────────────────────────────────┐
  │handle_sequencer_tick                     │
  │                                          │
  │run if:                                   │
  │- no other inflight request               │
  │                                          │
  │- requests latest sequencer block         │
  └──────────────────────────────────────────┘
                        │
                        │ - RPC Response
                        │
                        ▼
          ┌───────────────────────────┐
          │handle_sequencer_response  │
          │                           │
          │- process response         │
          │- convert if success       │
          └───────────────────────────┘
                        │
                        │  - Sequencer Block
                        │  - Sequencer height
                        │  - Validator information
                        │
                        ▼
┌───────────────────────────────────────────────┐
│convert_block_response_to_sequencer_block_data │
│                                               │
│- validate block height                        │
│- check if block proposed by own validator     │
│- convert block -> SequencerBlockData          │
└───────────────────────────────────────────────┘
                        │
                        │
                        │  - SequencerBlockData
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │handle_conversion_completed            │
    │                                       │
    │- push SequencerBlockData to gossipnet │
    │- add SequencerBlockData to DA queue   │
    └───────────────────────────────────────┘
```
<!-- markdownlint-disable MD013 -->
```text
                       ┌───────────────────────┐
                       │        DA Loop        │
                       └───────────────────────┘
                                   │
                                   │
                                   │ - DA Client
                                   │ - queued blocks
                                   │ - validator information
                                   ▼
             ┌──────────────────────────────────────────┐
             │submit_blocks_to_data_availability_layer  │
             │                                          │
             │- only runs if:                           │
             │  - DA connected                          │
             │  - DA queue has blocks                   │
             │  - No DA submission in flight            │
             └──────────────────────────────────────────┘
                                   │
                                   │
                                   │ - queued blocks
                                   │ - validator information
                                   │
                                   ▼
                  ┌────────────────────────────────┐
                  │DaClient.submit_all_blocks      │
                  │                                │
                  │- convert each block to blobs   │◀─┬───────────────────┐
                  │- submit blobs to DA            │  │                   │
                  │                                │  │                   │
                  └────────────────────────────────┘  │ - DA Blobs        │ - SequencerBlockData
                                   │                  │                   │ - Validator information
                                   │          ┌───────┘                   │
                ┌──────────────────┘          │                           ▼
                │                             │  ┌─────────────────────────────────────────────────┐
                │ - DA Response               │  │assemble_blobs_from_sequencer_block_data         │
                ▼                             │  │                                                 │
┌──────────────────────────────┐              │  │- create                                         │
│handle_submission_completed   │              │  │ - `SequencerNamespaceData`                      │
│                              │              └──│ - `RollupNamespaceData` for each rollup         │
│- update da height            │                 │- Wrap and sign all into `SignedNamespaceData`   │
│- log any errors              │                 │- Serialize `SignedNamespaceData`, wrap in blob  │
│                              │                 │                                                 │
└──────────────────────────────┘                 └─────────────────────────────────────────────────┘
```
<!-- markdownlint-enable MD013 -->
