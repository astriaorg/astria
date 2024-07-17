#!/bin/sh

get_evm_balance() {
  HEX_NUM=$(curl -X POST "$evm_url" -s -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$evm_to_address\", \"latest\"],\"id\":1}" -H 'Content-Type: application/json' | jq -r '.result')
  # strip 0x
  HEX_NUM=$(echo "$HEX_NUM" | sed 's/^0x//')
  # capitalize all lowercase letters
  HEX_NUM=$(echo "$HEX_NUM" | tr '[:lower:]' '[:upper:]')
  # print as integer
  echo "ibase=16; $HEX_NUM" | bc
}

addKeyForCelestiaAccount() {
  # add key for the celestia dev account using the mnemonic
  echo "Adding key for the celestia dev account..."
  echo "$celestia_dev_account_mnemonic" | celestia-appd keys add \
    "$celestia_dev_account_key_name" \
    --home "$home_dir" \
    --keyring-backend="$keyring_backend" \
    --recover
}

performIBCTransfer() {
  # perform ibc transfer
  echo "Performing IBC transfer..."
  celestia-appd tx ibc-transfer transfer \
    transfer \
    channel-0 \
    "$bridge_account_address_bech32" \
    53000utia \
    --memo="{\"rollupAddress\":\"$evm_to_address\"}" \
    --chain-id="$celestia_chain_id" \
    --node="$celestia_node_url" \
    --from="$celestia_dev_account_address" \
    --fees=26000utia \
    --yes \
    --log_level=debug \
    --home "$home_dir" \
    --keyring-backend="$keyring_backend"
}

initial_balance=$(get_evm_balance)

addKeyForCelestiaAccount
performIBCTransfer

# FIXME - should probably poll w/ timeout instead of sleeping?
sleep 30

final_balance=$(get_evm_balance)
expected_balance=$(echo "$initial_balance + 53000000000000000" | bc)
if [ "$(echo "$final_balance == $expected_balance" | bc)" -eq 0 ]; then
  echo "IBC Transfer failed!"
  echo "Expected balance $expected_balance, got $final_balance"
  exit 1
else
  echo "IBC Transfer successful!"
fi
