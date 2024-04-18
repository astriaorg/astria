#!/bin/sh

set -o errexit -o nounset

# if bridge config already exists then exit early
if [ -f "$home_dir/bridge/config.toml" ]; then
  exit 0
fi

celestia bridge init \
  --node.store "$home_dir/bridge" \
  --keyring.accname $validator_key_name \
  --keyring.backend $keyring_backend \
  --core.ip 127.0.0.1 \
  --core.rpc.port $celestia_app_host_port \
  --core.grpc.port $celestia_app_grpc_port \
  --gateway.port $bridge_host_port

sed -i 's/PeersLimit = 5/PeersLimit = 1/' $home_dir/bridge/config.toml
sed -i 's/Low = 50/Low = 0/' $home_dir/bridge/config.toml
