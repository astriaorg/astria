#!/bin/bash

set -o errexit -o nounset

DEFAULT_ACCOUNT="0xb0E31D878F49Ec0403A25944d6B1aE1bf05D17E1"
# use default account id if geth_local_account envar is not set
ACCOUNT=${geth_local_account:-$DEFAULT_ACCOUNT}

echo "Modifying genesis.json to allocate funds to $ACCOUNT"

mv /genesis.json $home_dir/genesis.bak.json

# use jq to replace alloc value in genesis.json with ACCOUNT envar
jq --arg accountId "$ACCOUNT" \
  '.alloc |= with_entries( if .key | startswith("0x") then .key = $accountId else . end )' \
  $home_dir/genesis.bak.json > $home_dir/genesis.json
