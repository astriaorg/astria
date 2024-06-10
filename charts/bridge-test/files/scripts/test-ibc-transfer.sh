#!/bin/sh

# add key for the celestia dev account using the mnemonic
echo "Adding key for the celestia dev account..."
echo "$celestia_dev_account_mnemonic" | celestia-appd keys add \
  "$celestia_dev_account_key_name" \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend" \
  --recover

# perform ibc transfer
echo "Performing IBC transfer..."
celestia-appd tx ibc-transfer transfer \
  transfer \
  channel-0 \
  "$bridge_account_address_bech32" \
  53000000utia \
  --memo="$evm_to_address" \
  --chain-id="$celestia_chain_id" \
  --node="$celestia_node_url" \
  --from="$celestia_dev_account_address" \
  --fees=420utia \
  --yes \
  --log_level=debug \
  --home "$home_dir" \
  --keyring-backend="$keyring_backend"
