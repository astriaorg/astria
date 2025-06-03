#!/bin/sh

set -o errexit -o nounset

# Start the celestia-app
exec celestia-appd start --home "${home_dir}" \
  --grpc.address "0.0.0.0:$celestia_app_grpc_port" \
  --rpc.grpc_laddr "tcp://0.0.0.0:$celestia_app_broadcast_grpc_port" \
  --rpc.laddr "tcp://0.0.0.0:$celestia_app_host_port" \
  --api.enable \
  --api.enabled-unsafe-cors \
  --grpc.enable \
  --grpc-web.enable \
  --force-no-bbr
