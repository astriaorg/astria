#!/bin/sh

set -o errexit -o nounset

exec celestia-appd start --home "${home_dir}" \
  --grpc.address "0.0.0.0:$celestia_app_grpc_port" \
  --rpc.grpc_laddr "tcp://0.0.0.0:9098" \
  --rpc.laddr "tcp://0.0.0.0:$celestia_app_host_port" \
  --api.enable \
  --api.enabled-unsafe-cors \
  --grpc.enable \
  --grpc-web.enable \
  --force-no-bbr \
  --log_level "debug" \
  --minimum-gas-prices "0.002utia"
