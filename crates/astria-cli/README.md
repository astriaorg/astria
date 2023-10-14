# Astria CLI

Astria CLI is a command line tool for interacting with the Sequencer network
and for managing your own rollup deployments. You can create a
Sequencer account, check balances, generate rollup deployment configurations,
deploy rollups, and more.

## Dependencies

* rust - https://www.rust-lang.org/tools/install
* docker - https://docs.docker.com/get-docker/
* kubectl - https://kubernetes.io/docs/tasks/tools/
* kind - https://kind.sigs.k8s.io/docs/user/quick-start/#installation 
* helm - https://helm.sh/docs/intro/install/

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

# create a rollup config
./target/release/astria-cli rollup config create \
  --use-tty \
  --log-level DEBUG \
  --rollup.name somerollupname \
  --rollup.chain-id somechainid \
  --rollup.network-id 42 \
  --rollup.skip-empty-blocks \
  --sequencer.initial-block-height 1 \
  --sequencer.websocket wss://rpc.sequencer.dusk-1.devnet.astria.org/websocket \
  --sequencer.rpc https://rpc.sequencer.dusk-1.devnet.astria.org \
  --rollup.genesis-accounts 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:100000000000000000000
  
# edit config
./target/release/astria-cli rollup config edit \
  --config somerollupname-rollup-config.yaml
  <KEY> <VALUE>

# delete config
./target/release/astria-cli rollup config delete \
  --config somerollupname-rollup-config.yaml

# create deployment from config
# FAUCET_PRIVATE_KEY - 64 character hex string. private key of account used to
#  fund the faucet. This will often be the private key of an address used in
#  the `rollup.genesis-accounts` argument for `rollup config create` above.
# SEQUENCER_PRIVATE_KEY - private key of account used to wrap transactions for
#  submission to the sequencer.
./target/release/astria-cli rollup deployment create \
  --config somerollupname-rollup-config.yaml \
  --faucet-private-key <FAUCET_PRIVATE_KEY> \
  --sequencer-private-key <SEQUENCER_PRIVATE_KEY>
  
# NOTE - you can also run `deployment create` with `--dry-run` to see the
#  generated k8s yaml without actually creating the deployment
  
# list deployments
./target/release/astria-cli rollup deployment list

# delete deployment
./target/release/astria-cli rollup deplyoment delete \
  --config somerollupname-rollup-config.yaml
  
# get balance of account on Sequencer
./target/release/astria-cli sequencer balance get <ADDRESS> \
  --sequencer_url <SEQUENCER_URL>
  
# get latest block height of Sequencer
./target/release/astria-cli sequencer blockheight get \
  --sequencer_url <SEQUENCER_URL>
```
