#!/bin/sh

set -o errexit -o nounset

celestia-appd init "$chainid" \
  --chain-id "$chainid" \
  --home "$home_dir"

celestia-appd keys add \
  "$validator_key_name" \
  --keyring-backend="$keyring_backend" \
  --home "$home_dir"

validator_key=`celestia-appd keys show "$validator_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir"`
celestia-appd add-genesis-account \
  "$validator_key" \
  --home "$home_dir" \
  "$coins"

celestia-appd gentx \
  "$validator_key_name" \
  "$validator_stake" \
  --keyring-backend="$keyring_backend" \
  --chain-id "$chainid" \
  --home "$home_dir" \
  --orchestrator-address "$validator_key" \
  --evm-address "$evm_address"

celestia-appd collect-gentxs --home "$home_dir"
