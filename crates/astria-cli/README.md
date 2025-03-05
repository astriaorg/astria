# Astria CLI

Astria CLI is a command line tool for interacting with the Sequencer network.
You can create a Sequencer account, check balances, and more.

## Dependencies

* rust - <https://www.rust-lang.org/tools/install>

## Setup

## Building

```sh
cargo build --release
```

## Running

```sh
# from monorepo root
./target/release/astria-cli --help

# examples:

# create account on Sequencer
./target/release/astria-cli sequencer account create

# get balance of account on Sequencer
./target/release/astria-cli sequencer balance get <ADDRESS> \
  --sequencer_url <SEQUENCER_URL>

# get latest block height of Sequencer
./target/release/astria-cli sequencer blockheight get \
  --sequencer_url <SEQUENCER_URL>

# submit a rollup transaction to the Sequencer
./target/release/astria-cli sequencer submit-rollup-tx \
  --rollup-id <ROLLUP_ID> \
  --data <HEX_DATA> \
  --private-key <PRIVATE_KEY> \
  --sequencer-url <SEQUENCER_URL> \
  --sequencer.chain-id <CHAIN_ID>
```
