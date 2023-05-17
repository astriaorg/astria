#!/bin/sh

set -o errexit -o nounset

metro init "$chainid" \
  --chain-id "$chainid" \
  --home "$home_dir"

metro keys add "$validator_key_name" \
  --keyring-backend="$keyring_backend" \
  --home "$home_dir"

validator_key=`metro keys show "$validator_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir"`
metro add-genesis-account "$validator_key" "$coins" \
  --home "$home_dir"

metro gentx "$validator_key_name" "$validator_stake" \
  --keyring-backend="$keyring_backend" \
  --chain-id "$chainid" \
  --orchestrator-address "$validator_key" \
  --evm-address "$evm_address" \
  --home "$home_dir"

metro collect-gentxs \
  --home "$home_dir"
