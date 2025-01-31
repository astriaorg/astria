# Astria native oracle protocol

The Astria sequencer has an oracle protocol built into its consensus. Oracle
data can be provided by the network for each block that has sufficient voting
power (>2/3) backing the oracle values. Currently, the only oracle data provided
is price data for specific currency pairs, eg. BTC/USD, ETH/USD, TIA/USD. The
oracle protocol is based off Skip's [Connect](https://github.com/skip-mev/connect/tree/main)
protocol. Astria uses Skip's oracle sidecar to fetch price data.

### High level overview

Astria uses CometBFT for consensus, which communicates with the application
logic using [ABCI++](https://docs.cometbft.com/v0.37/spec/abci/abci++_basic_concepts#consensusblock-execution-methods).
During each consensus round, validators gossip "vote extensions" (VEs), which can be
any arbitrary data. In our case, validators put price data in vote extensions.
Nodes get the price data from Skip's oracle sidecar process, which fetches and
returns prices for each pair in the app state's market map.

In some round, validators perform basic validation on the VEs received from peers, such as
deserialization and length, but not on the contents. Since vote extensions are
gossiped on the p2p network, each validator has a different, local view of the
vote extensions it received from peers during that round. Any vote extensions
that failed basic validation are excluded from this local view. 

During the following round, the block proposer proposes a canonical set of vote extensions
(valid VEs the proposer saw during the previous round). This set is validated by 
the network during consensus. If the set is finalized, and >2/3 voting power contributed 
prices to this set, prices are updated in the application's state for that block.

The overall price for a currency pair is calculated by using the voting-power-weighted
median of the prices posted by each validator. This way, one malfunctioning validator should
not be able to affect the resulting aggregated price.

Since the sequencer network (along with Celestia, for DA) is used by rollups for 
transaction data and ordering, rollups are also able to access the price data 
committed by the sequencer and include them as rollup transactions. Rollups would 
need to opt-in to use the oracle data and implement how the data is to be stored; for 
example, an EVM rollup (such as Astria's Flame) can store the oracle data in a smart
contract.

## Implementation

### Types

Validators place the following (proto-encoded) type inside vote extensions:

```rust
pub struct OracleVoteExtension {
    pub prices: IndexMap<CurrencyPairId, Price>,
}
```

`OracleVoteExtension` contains a map of currency pair IDs to prices, where the currency pair ID
is mapped to a full currency pair inside the sequencer application. The IDs are assigned starting 
from 0, incrementing by 1 for each pair added. IDs cannot repeat. Pairs can be added to state 
either in genesis, or via a `PriceFeed::Oracle` action. If a pair is removed, its ID cannot be re-used.

The `SequencerBlock` type, which is written to DA, contains the following oracle-related fields:

```rust
pub struct SequencerBlock {
    /// fields omitted 
    /// ...

    /// The extended commit info for the block, if vote extensions were enabled at this height.
    ///
    /// This is verified to be of the form `ExtendedCommitInfoWithCurrencyPairMapping` when the
    /// type is constructed, but is left as `Bytes` so that it can be verified against the
    /// `data_hash` using the `extended_commit_info_proof` (as re-encoding the protobuf type
    /// may not be deterministic).
    extended_commit_info: Option<Bytes>,
    /// The proof that the extended commit info is included in the cometbft block data (if it
    /// exists), specifically the third item in the data field.
    extended_commit_info_proof: Option<merkle::Proof>,
}
```

where `ExtendedCommitInfoWithCurrencyPairMapping` is the following:

```rust
pub struct ExtendedCommitInfoWithCurrencyPairMapping {
    // the entire set of vote extensions finalized in the sequencer block.
    pub extended_commit_info: ExtendedCommitInfo,
    // mapping of currency pair ID (since vote extensions contain only ID->price)
    // to the currency pair and its price decimals.
    pub id_to_currency_pair: IndexMap<CurrencyPairId, CurrencyPairInfo>,
}

pub struct CurrencyPairInfo {
    pub currency_pair: CurrencyPair,
    pub decimals: u64,
}
```

The `ExtendedCommitInfoWithCurrencyPairMapping` along with the `extended_commit_info_proof` contains
all the information required for the conductor to verify that the `extended_commit_info` was 
finalized by the sequencer and to reconstruct the price data.

The conductor wraps each piece of data before sending it to the rollup:

```rust
pub enum RollupData {
    SequencedData(Bytes),
    Deposit(Box<Deposit>),
    OracleData(Box<OracleData>),
}
```

The price data is set to the rollup as a `RollupData::OracleData` variant, which the 
rollup can then decode and handle how it wishes.

### Sequencer application

There are four main places where oracle data is processed in the sequencer app: the vote extension
dissemination during a consensus round, `prepare_proposal` of the following block, `process_proposal`
of the following block, and `finalize_block`.

#### Vote extension process

During a consensus round, cometbft will call into the application via ABCI to get the vote extension
data via `extend_vote`, and to verify vote extensions of peers via `verify_vote_extension`.

In `extend_vote`, the node will call the oracle sidecar and fetch the latest prices for all the pairs
in the current state market map. It will then encode these prices and return this to cometbft, which
will put the data in a vote extension. The VE is then gossiped to other validators.

When a node receives a vote extension from a peer, cometbft will call `verify_vote_extension`.
The app performs basic validation on the vote extension, ensuring it doesn't exceed the maximum byte 
size, that it's a valid `OracleVoteExtension`, and the number of prices doesn't exceed 
the maximum. If the VE passes this check, it's added to the node's local VE view for that round. 

#### `prepare_proposal`

The proposer of a block receives all the vote extensions in its local view from
the previous round inside `prepare_proposal`, specifically the `local_last_commit`
field which contains the previous round's commit (>2/3 voting power set of votes 
from validators) as well as their vote extensions. The proposer then checks if >2/3
voting power submitted valid oracle data vote extensions. If not, the proposer
omits the `extended_commit_info` (proposed canonical extended vote set) from the block. 
Otherwise, it includes it in the proposed block at a specific block data index.

Note that in the case where not enough validators partake in the oracle protocol,
liveness of the chain is unaffected. Blocks continue to be produced, but oracle 
prices are not updated.

#### `process_proposal`



#### `finalize_block`

### Conductor

### EVM rollup 
