# Astria native oracle protocol

The Astria sequencer has an oracle protocol built into its consensus. Oracle
data can be provided by the network for each block that has sufficient voting
power (>2/3) backing the oracle values. Currently, the only oracle data provided
is price data for specific currency pairs, eg. BTC/USD, ETH/USD, TIA/USD. The
oracle protocol is based off Skip's [Connect](https://github.com/skip-mev/connect/tree/main)
protocol. Astria uses Skip's oracle sidecar to fetch price data.

## High level overview

Astria uses CometBFT for consensus, which communicates with the application
logic using [ABCI++](https://docs.cometbft.com/v0.38/spec/abci/abci++_basic_concepts#consensusblock-execution-methods).
During each consensus round, validators gossip "vote extensions" (VEs), which can
be any arbitrary data. In our case, validators put price data in vote extensions.
Nodes get the price data from Skip's oracle sidecar process, which fetches and
returns prices for each pair in the app state's market map.

In some round, validators perform basic validation on the VEs received from peers,
such as deserialization and length, but not on the contents. Since vote extensions
are gossiped on the p2p network, each validator has a different, local view of the
vote extensions it received from peers during that round. Any vote extensions
that failed basic validation are excluded from this local view.

During the following round, the block proposer proposes a canonical set of vote extensions
(valid VEs the proposer saw during the previous round). This set is validated by
the network during consensus. If the set is finalized, and >2/3 voting power contributed
prices to this set, prices are updated in the application's state for that block.

The overall price for a currency pair is calculated by using the voting-power-weighted
median of the prices posted by each validator. This way, one malfunctioning validator
should not be able to significantly affect the resulting aggregated price.

Since the sequencer network (along with Celestia, for DA) is used by rollups for
transaction data and ordering, rollups are also able to access the price data
committed by the sequencer and include them as rollup transactions. Rollups would
need to opt-in to use the oracle data and implement how the data is to be stored;
for example, an EVM rollup (such as Astria's Flame) can store the oracle data in
a smart contract.

## Implementation

### Types

Validators place the following (proto-encoded) type inside vote extensions:

```rust
pub struct OracleVoteExtension {
    pub prices: IndexMap<CurrencyPairId, Price>,
}
```

`OracleVoteExtension` contains a map of currency pair IDs to prices, where the
currency pair ID is mapped to a full currency pair inside the sequencer application.
The IDs are assigned starting from 0, incrementing by 1 for each pair added. IDs
cannot repeat. Pairs can be added to state either in genesis, or via a `PriceFeed::Oracle`
action. If a pair is removed, its ID cannot be re-used.

The `SequencerBlock` type, which is written to DA, contains the following oracle-related
fields:

```rust
pub struct SequencerBlock {
    /// fields omitted 
    /// ...

    /// The extended commit info for the block, if vote extensions were enabled
    /// at this height.
    ///
    /// This is verified to be of the form `ExtendedCommitInfoWithCurrencyPairMapping`
    /// when the type is constructed, but is left as `Bytes` so that it can be
    /// verified against the `data_hash` using the `extended_commit_info_proof`
    /// (as re-encoding the protobuf type may not be deterministic).
    extended_commit_info: Option<Bytes>,
    /// The proof that the extended commit info is included in the cometbft block
    /// data (if it exists), specifically the third item in the data field.
    extended_commit_info_proof: Option<merkle::Proof>,
}
```

where `ExtendedCommitInfoWithCurrencyPairMapping` is the following:

```rust
pub struct ExtendedCommitInfoWithCurrencyPairMapping {
    // The entire set of vote extensions finalized in the sequencer block.
    pub extended_commit_info: ExtendedCommitInfo,
    // Mapping of currency pair ID (since vote extensions contain only ID->price)
    // to the currency pair and its price decimals.
    pub id_to_currency_pair: IndexMap<CurrencyPairId, CurrencyPairInfo>,
}

pub struct CurrencyPairInfo {
    pub currency_pair: CurrencyPair,
    pub decimals: u64,
}
```

The `ExtendedCommitInfoWithCurrencyPairMapping` along with the `extended_commit_info_proof`
contains all the information required for the conductor to verify that the `extended_commit_info`
was finalized by the sequencer and to reconstruct the price data.

The conductor wraps each piece of data before sending it to the rollup:

```rust
pub enum RollupData {
    SequencedData(Bytes),
    Deposit(Box<Deposit>),
    OracleData(Box<OracleData>),
}
```

The price data is sent to the rollup as a `RollupData::OracleData` variant, which
the rollup can then decode and handle how it wishes.

### Sequencer application

There are four main places where oracle data is processed in the sequencer app:
the vote extension dissemination during a consensus round, `prepare_proposal` of
the following block, `process_proposal` of the following block, and `finalize_block`.

#### Vote extension process

During a consensus round, cometbft will call into the application via ABCI to get
the vote extension data via [`extend_vote`](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/vote_extension.rs#L85),
and to verify vote extensions of peers
via [`verify_vote_extension`](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/vote_extension.rs#L117).

In `extend_vote`, the node will call the oracle sidecar and fetch the latest prices
for all the pairs in the current state market map. It will then encode these prices
and return this to cometbft, which will put the data in a vote extension. The VE
is then gossiped to other validators.

When a node receives a vote extension from a peer, cometbft will call `verify_vote_extension`.
The app performs basic validation on the vote extension, ensuring it doesn't exceed
the maximum byte size, that it's a valid `OracleVoteExtension`, and the number of
prices doesn't exceed the maximum. If the VE passes this check, it's added to the
node's local VE view for that round.

#### `prepare_proposal`

The proposer of a block receives all the vote extensions in its local view from
the previous round inside [`prepare_proposal`](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/vote_extension.rs#L210),
specifically the `local_last_commit`
field which contains the previous round's commit (>2/3 voting power set of votes
from validators) as well as their vote extensions. The proposer then checks if >2/3
voting power submitted valid oracle data vote extensions. If not, the proposer
omits the `extended_commit_info` (proposed canonical extended vote set) from the
block. Otherwise, it includes it in the proposed block at a specific block data index.

Note that in the case where not enough validators partake in the oracle protocol,
liveness of the chain is unaffected. Blocks continue to be produced, but oracle
prices are not updated.

#### `process_proposal`

Other validators receive the proposed block in `process_proposal`, which calls
[`validate_proposal`](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/vote_extension.rs#L271).
If it contains
non-empty extended commit info, it validates it by checking that the signature
for each vote extension is valid, and corresponds to an actual validator. It checks
that each VE is itself a valid `OracleVoteExtension`. It also checks that >2/3
voting power submitted valid VEs. If these checks pass, then the validator
accepts the proposal (assuming the rest of the block is also valid), otherwise,
it rejects the proposal. If the block's extended commit info is empty, then we
assume the proposer did not receive the necessarily amount of VEs to create a
valid VE set, and skip oracle validation to ensure liveness.

#### `finalize_block`

When a block with an extended commit info set is [finalized](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/mod.rs#L1093),
the node uses the price
data inside the vote extensions to [calculate updated prices](https://github.com/astriaorg/astria/blob/b2083b4a82195dc9be1e85f31cea14c724b8b4ec/crates/astria-sequencer/src/app/vote_extension.rs#L577)
for each currency pair.
For each currency pair, the node takes the validator-power-weighted median of
the prices to calculate the new price, which is then stored in the app state.

### Conductor

The conductor receives the full extended commit info set either from the sequencer
or from DA via `SequencerBlock`. The conductor verifies that this was actually
included in the block data using `extended_commit_info_proof`, which is a merkle
proof into the block's `data_hash`. The conductor does not need to perform the
entire validation that `process_proposal` does, as if the extended commit info
was included in a block, it was already validated by >2/3 voting power of the network.

The conductor then calculates the prices for each pair using the same algorithm
as `finalize_block` and formats them into `RollupData::OracleData`, which it
sends to the rollup execution node via the execution API, prepended to the
collection of transactions. The price feed data is prepended as it's anticipated
that rollups will usually want to apply the updated price data before executing
the rest of the transactions.
