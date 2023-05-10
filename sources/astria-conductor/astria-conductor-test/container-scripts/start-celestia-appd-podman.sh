#!/bin/sh

set -o errexit -o nounset

sed -i'.bak' "s#\"tcp://127.0.0.1:26657\"#\"tcp://0.0.0.0:$celestia_app_host_port\"#g" $home_dir/config/config.toml
sed -i'.bak' 's/timeout_commit = "25s"/timeout_commit = "1s"/g' $home_dir/config/config.toml
sed -i'.bak' 's/timeout_propose = "10s"/timeout_propose = "1s"/g' $home_dir/config/config.toml

# Start the celestia-app
exec celestia-appd start --home "${home_dir}"
