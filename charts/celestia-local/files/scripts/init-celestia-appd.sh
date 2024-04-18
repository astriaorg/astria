#!/bin/sh

set -o errexit -o nounset

rm -rf $home_dir/*

celestia-appd init "$chainid" \
  --chain-id "$chainid" \
  --home "$home_dir"

if [ -n "$validator_mnemonic" ]; then
  echo $validator_mnemonic | celestia-appd keys add \
    "$validator_key_name" \
    --home "$home_dir" \
    --keyring-backend="$keyring_backend" \
    --recover
else
  celestia-appd keys add \
    "$validator_key_name" \
    --keyring-backend="$keyring_backend" \
    --home "$home_dir"
fi

validator_key=$(celestia-appd keys show "$validator_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir")
celestia-appd add-genesis-account \
  "$validator_key" \
  --home "$home_dir" \
  "$coins"

celestia-appd gentx \
  "$validator_key_name" \
  "$validator_stake" \
  --keyring-backend="$keyring_backend" \
  --chain-id "$chainid" \
  --home "$home_dir"

celestia-appd collect-gentxs --home "$home_dir"

# Enable transaction indexing
sed -i'.bak' 's#"null"#"kv"#g' "${home_dir}"/config/config.toml
# Persist ABCI responses
sed -i'.bak' 's#discard_abci_responses = true#discard_abci_responses = false#g' "${home_dir}"/config/config.toml
# Override the VotingPeriod from 1 week to 1 minute
sed -i'.bak' 's#"604800s"#"60s"#g' "${home_dir}"/config/genesis.json
