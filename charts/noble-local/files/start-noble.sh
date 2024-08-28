#!/bin/sh

set -o errexit -o nounset

KEYRING="--keyring-backend=$keyring_backend"

{
  # create keys
  sleep 2
  nobled --home "$home_dir" "$KEYRING" keys add masterminter
  nobled --home "$home_dir" "$KEYRING" keys add mintercontroller
  nobled --home "$home_dir" "$KEYRING" keys add minter
  nobled --home "$home_dir" "$KEYRING" keys add blacklister
  nobled --home "$home_dir" "$KEYRING" keys add pauser
  nobled --home "$home_dir" "$KEYRING" keys add user

  # fund accounts
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show masterminter -a)" 50ustake -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show mintercontroller -a)" 50ustake -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show minter -a)" 50ustake -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show blacklister -a)" 50ustake -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show pauser -a)" 50ustake -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx bank send tf1_owner "$(nobled --home "$home_dir" "$KEYRING" keys show user -a)" 50ustake -y

  sleep 2

  # delegate privileges
  nobled --home "$home_dir" "$KEYRING" \
    tx tokenfactory update-master-minter "$(nobled --home "$home_dir" "$KEYRING" keys show masterminter -a)" --from tf1_owner -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx tokenfactory configure-minter-controller "$(nobled --home "$home_dir" "$KEYRING" keys show mintercontroller -a)" "$(nobled --home "$home_dir" "$KEYRING" keys show minter -a)" --from masterminter -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx tokenfactory configure-minter "$(nobled --home "$home_dir" "$KEYRING" keys show minter -a)" 1000$TF1_MINTING_BASEDENOM --from mintercontroller -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx tokenfactory update-blacklister "$(nobled --home "$home_dir" "$KEYRING" keys show blacklister -a)" --from tf1_owner -y

  sleep 2
  nobled --home "$home_dir" "$KEYRING" \
    tx tokenfactory update-pauser "$(nobled --home "$home_dir" "$KEYRING" keys show pauser -a)" --from tf1_owner -y
} &

exec nobled start --home "${home_dir}" \
  --grpc.enable \
  --grpc.address "0.0.0.0:$noble_grpc_port" \
  --grpc-web.enable \
  --grpc-web.address "0.0.0.0:$noble_grpc_web_port" \
  --log_level debug
