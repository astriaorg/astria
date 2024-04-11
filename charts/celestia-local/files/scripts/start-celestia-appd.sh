#!/bin/sh

set -o errexit -o nounset

# Start the celestia-app
{
  # Wait for block 1
  sleep 15

  VALIDATOR_ADDRESS=$(celestia-appd keys show $validator_key_name --home "$home_dir" --bech val --address)
  celestia-appd keys export $validator_key_name \
    --keyring-backend="$keyring_backend" \
    --home "$home_dir" --unsafe --unarmored-hex \
    > "/home/celestia/data.hex" 2>&1 \
    << EOF
y
EOF

  echo "Registering an EVM address for validator..."
  celestia-appd tx qgb register \
    $VALIDATOR_ADDRESS \
    $evm_address \
    --from $validator_key_name \
    --home $home_dir \
    --fees 30000utia \
    --broadcast-mode block \
    --yes \
    &> /dev/null # Hide output to reduce terminal noise

  echo "Registered EVM address."
} &

exec celestia-appd start --home "${home_dir}" \
  --grpc.address "0.0.0.0:$celestia_app_grpc_port" \
  --rpc.laddr "tcp://0.0.0.0:$celestia_app_host_port" \
  --api.enable \
  --grpc.enable \
  --grpc-web.enable
