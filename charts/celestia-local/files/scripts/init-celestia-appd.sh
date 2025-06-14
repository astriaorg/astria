#!/bin/sh

set -o errexit -o nounset

if [ -f "$home_dir/config/config.toml" ]; then
  exit 0
fi

celestia-appd init "$chainid" \
  --chain-id "$chainid" \
  --home "$home_dir"

echo "$validator_mnemonic" | celestia-appd keys add \
  "$validator_key_name" \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend" \
  --recover

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
  --fees "$fees" \
  --home "$home_dir"

# add ibc account
echo "$relayer_one_mnemonic" | celestia-appd keys add \
  "$relayer_one_account_key_name" \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend" \
  --recover
relayer_one_account_key=$(celestia-appd keys show "$relayer_one_account_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir")
celestia-appd add-genesis-account \
  "$relayer_one_account_key" \
  --home "$home_dir" \
  "$coins"

# add relayer two
echo "$relayer_two_mnemonic" | celestia-appd keys add \
  "$relayer_two_account_key_name" \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend" \
  --recover
relayer_two_account_key=$(celestia-appd keys show "$relayer_two_account_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir")
celestia-appd add-genesis-account \
  "$relayer_two_account_key" \
  --home "$home_dir" \
  "$coins"

# add account used for development and testing
echo "$dev_account_mnemonic" | celestia-appd keys add \
  "$dev_account_key_name" \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend" \
  --recover
dev_account_key=$(celestia-appd keys show "$dev_account_key_name" -a --keyring-backend="$keyring_backend" --home "$home_dir")
celestia-appd add-genesis-account \
  "$dev_account_key" \
  --home "$home_dir" \
  "$coins"

celestia-appd collect-gentxs --home "$home_dir"

# Enable transaction indexing
sed -i'.bak' 's#"null"#"kv"#g' "${home_dir}"/config/config.toml
# Persist ABCI responses
sed -i'.bak' 's#discard_abci_responses = true#discard_abci_responses = false#g' "${home_dir}"/config/config.toml
# Override the VotingPeriod from 1 week to 1 minute
sed -i'.bak' 's#"604800s"#"60s"#g' "${home_dir}"/config/genesis.json
# Set the CommitTimeout to 5 second
sed -i'.bak' 's#timeout_commit = "11s"#timeout_commit = "5s"#g' "${home_dir}"/config/config.toml

if $fast; then
  sed -i'.bak' 's#timeout_commit = "5s"#timeout_commit = "1s"#g' "${home_dir}"/config/config.toml
fi
