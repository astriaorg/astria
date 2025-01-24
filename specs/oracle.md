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
