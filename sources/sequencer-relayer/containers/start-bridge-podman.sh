#!/bin/bash

set -o errexit -o nounset -o pipefail

genesis_hash=$(curl -s -S -X GET "http://127.0.0.1:26657/block?height=1" | jq -r '.result.block_id.hash')
if [ -z "$genesis_hash" ] 
then
  echo "did not receive genesis hash from celestia; exiting"
  exit 1
else
  echo "genesis hash received: $genesis_hash"
fi

export CELESTIA_CUSTOM="test:$genesis_hash"
  # --p2p.network "test:$celestia_custom"
export GOLOG_LOG_LEVEL="debug"
exec ./celestia bridge start \
  --node.store "$home_dir/bridge" \
  --gateway \
  --keyring.accname "$validator_key_name"