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
  --genesis-accounts 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:100000000000000000000

# deploy this config
./target/release/astria-cli rollup config deploy \
  --filename somerollupname-rollup-config.yaml \
  --faucet-private-key <FAUCET_PRIVATE_KEY> \
  --sequencer-private_key <SEQUENCER_PRIVATE_KEY>

# delete the deployment that was created from the config
./target/release/astria-cli rollup config delete --filename somerollupname-rollup-config.yaml
```
