# Rust bindings for Astria Bridge Contracts

Rust bindings for Astria's solidity contracts at
[astriaorg/astria-bridge-contracts](https://github.com/astriaorg/astria-bridge-contracts).
The repository is tracked by the
[./astria-bridge-contracts](./astria-bridge-contracts) submodule.

The bindings are generated using the
[solidity compiler tool](../../tools/solidity-compiler).

If the upstream repository and its contracts have changed, update the submodule
and re-generate the bindings like so:

```sh
# inside crates/astria-bridge-contracts
cd ./astria-bridge-contract
git checkout <commit-ish>
# navigate to root of repository
cd ../../../
just compile-solidity-contracts
git add .
git commit -m "chore(bridge-contracts)!: bumped bridge contracts
```
