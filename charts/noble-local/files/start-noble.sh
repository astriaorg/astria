#!/bin/sh

set -o errexit -o nounset

KEYRING="--keyring-backend=$keyring_backend"

exec nobled start --home "${home_dir}"\
  --grpc.enable \
  --grpc.address "0.0.0.0:$noble_grpc_port" \
  --grpc-web.enable \
  --log_level debug
