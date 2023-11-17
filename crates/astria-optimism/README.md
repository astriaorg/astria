# astria-optimism

A small library which contains functionality for reading, making, and deriving
 deposits from an Ethereum L1 where the OP-Stack contracts are deployed.

Specifically, we can:

- read deposit events from an `OptimismPortal.sol` contract on L1
- make deposits on the L1 to an `OptimismPortal.sol` contract
- turn deposit events on L1 into L2 deposit transactions (which cause
 funds to be minted on an OP-Stack L2 execution node)

## tests

You will need solc 0.8.15 and foundry installed. I recommend using
[solc-select](https://github.com/crytic/solc-select) for managing
solc installs and versions.

```rust
cargo test
```

This library contains a test contract, `MockOptimismPortal.sol`, which
copy-pastes the relevant functionality from
[OptimismPortal.sol](https://github.com/ethereum-optimism/optimism/blob/9a13504bb1f302ca9d412589aac18d589c055f16/packages/contracts-bedrock/src/L1/OptimismPortal.sol)
for testing purposes.
