#!/bin/bash

set -o errexit -o nounset -o pipefail

function set_token() {
  # NOTE - this is a hack to get the token to the token-server directory.
  TOKEN=$(celestia bridge auth admin \
    --node.store "$home_dir/bridge" \
    --keyring.accname $validator_key_name)

  # Busybox's httpd doesn't support url rewriting, so it's not simple to server another file.
  # To support an ingress rule path of `/`, we write the token to index.html, which httpd serves by default.
  mkdir -p "$home_dir"/token-server
  echo "$TOKEN" >"$home_dir"/token-server/index.html
}

function get_genesis() {
  local genesis_hash=$(curl -s -S -X GET "http://127.0.0.1:$celestia_app_host_port/block?height=1" | jq -r '.result.block_id.hash')
  echo "$genesis_hash"
}

function wait_for_genesis() {
  local genesis_hash=$(get_genesis)

  while [ "$genesis_hash" = "null" ]; do
    sleep 1
    genesis_hash=$(get_genesis)
  done

  echo "$genesis_hash"
}

# only create token if it does not already exist
# FIXME - would it be bad to get a new token on every start?
if [ ! -f "$home_dir"/token-server/index.html ]; then
  set_token
fi

echo "waiting for genesis hash from celestia..."
genesis_hash=$(wait_for_genesis)
echo "genesis hash received: $genesis_hash"

export CELESTIA_CUSTOM="test:$genesis_hash"
export GOLOG_LOG_LEVEL="debug"

# fixes: keystore: permissions of key 'p2p-key' are too relaxed: required: 0600, got: 0660
# FIXME - how are the perms getting changed from the first start which works fine?
# NOTE - using `find` here to avoid chmod'ing the keyring-test directory
find "$home_dir/bridge/keys" -type f -exec chmod 0600 {} \;

echo "staring bridge!"
exec celestia bridge start \
  --node.store "$home_dir/bridge" \
  --core.ip 0.0.0.0 \
  --core.rpc.port "$celestia_app_host_port" \
  --core.grpc.port "$celestia_app_grpc_port" \
  --gateway \
  --gateway.addr 0.0.0.0 \
  --gateway.port "$bridge_host_port" \
  --rpc.addr 0.0.0.0 \
  --rpc.port "$bridge_rpc_port" \
  --keyring.accname "$validator_key_name" \
  --log.level "debug" \
  --log.level.module "share/discovery:error"
