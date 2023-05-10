#!/bin/sh

set -o errexit -o nounset

./celestia bridge init \
  --node.store "$home_dir/bridge" \
  --core.ip 127.0.0.1
cp -r "$home_dir/keyring-test" "$home_dir/bridge/keys/"

# must replace the app port used in the bridge config.toml
sed -i'.bak' "s#Port = \"26657\"#Port = \"$celestia_app_host_port\"#g" $home_dir/bridge/config.toml
sed -i'.bak' "s#Port = \"26659\"#Port = \"$bridge_host_port\"#g" $home_dir/bridge/config.toml
