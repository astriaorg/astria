#!/usr/bin/env bash

apt-get update
apt-get install -y curl jq

./celestia bridge init --node.store /bridge
/wait-for-it.sh 192.167.10.10:26657 -t 90 -- \
  curl -s http://192.167.10.10:26657/block?height=1 | jq '.result.block_id.hash' | tr -d '"' > genesis.hash

curl -s http://192.167.10.10:26657/block_by_hash?hash=0x`cat genesis.hash`
echo  # newline

export CELESTIA_CUSTOM=test:`cat genesis.hash`
echo $CELESTIA_CUSTOM
./celestia bridge start \
  --node.store /bridge --gateway \
  --core.ip 192.167.10.10 \
  --keyring.accname validator
