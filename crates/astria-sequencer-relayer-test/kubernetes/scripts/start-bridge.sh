#!/bin/sh -x

set -o errexit -o nounset -o pipefail

if genesis_hash=$(curl -s -S -X GET "http://127.0.0.1:26657/block?height=1" | jq -er '.result.block_id.hash');
then
  : "genesis hash received successfully"
else
  echo "did not receive genesis hash from celestia; exiting"
  exit 1
fi

echo "using genesis hash: $genesis_hash"

export GOLOG_LOG_LEVEL="debug"
export CELESTIA_CUSTOM="test:$genesis_hash"
exec celestia bridge start \
  --node.store "$home_dir/bridge" \
  --gateway \
  --keyring.accname "$validator_key_name" \ 
  --rpc.port 26690
