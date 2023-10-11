# Astria CLI

## Dependencies

- kubectl

## Building

```sh
cargo build --release
```

## Running

```sh
# from monorepo root
./target/release/astria-cli --help

# examples:

# create a rollup config
./target/release/astria-cli rollup config create \
  --use-tty \
  --log-level DEBUG \
  --rollup.name somerollupname \
  --rollup.chain-id 0x1234 \
  --rollup.network-id 42 \
  --rollup.skip-empty-blocks \
  --sequencer.initial-block-height 1 \
  --sequencer.websocket ws://sequencer-service:26657/websocket \
  --sequencer.rpc http://sequencer-service:26657 \
  --celestia.full-node-url http://celestia-service:26658 \
  --rollup.genesis-accounts 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:100000000000000000000
  
# edit config
./target/release/astria-cli rollup config edit \
  --config somerollupname-rollup-config.yaml
  <KEY> <VALUE>

# delete config
./target/release/astria-cli rollup config delete \
  --config somerollupname-rollup-config.yaml

# create deployment from config
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
  
# create account on Sequencer
./target/release/astria-cli sequencer account create

# get balance of account on Sequencer
./target/release/astria-cli sequencer balance get <ADDRESS> \
  --sequencer_url <SEQUENCER_URL>
  
# get latest block height of Sequencer
./target/release/astria-cli sequencer blockheight get \
  --sequencer_url <SEQUENCER_URL>
```
