#!/bin/bash

DEFAULT_ACCOUNT_ID="0xb0E31D878F49Ec0403A25944d6B1aE1bf05D17E1"

# use default account id if ACCOUNT_ID envar is not set
ACCOUNT_ID=${ACCOUNT_ID:-$DEFAULT_ACCOUNT_ID}

# use jq to replace alloc value in genesis.json with ACCOUNT_ID envar
mv genesis.json genesis.bak.json
jq --arg accountId "$ACCOUNT_ID" \
  '.alloc |= with_entries( if .key | startswith("0x") then .key = $accountId else . end )' \
  genesis.bak.json > genesis.json

geth --datadir ~/.astriageth/ init genesis.json
geth --datadir ~/.astriageth/ --http --http.addr "0.0.0.0" --http.port=8545 \
  --ws --ws.addr "0.0.0.0" --ws.port=8545 --networkid=1337 --http.corsdomain='*' --ws.origins='*' \
  --grpc --grpc.addr "0.0.0.0" --grpc.port 50051 \
  --metro.addr "192.167.10.40" --metro.port 9090
