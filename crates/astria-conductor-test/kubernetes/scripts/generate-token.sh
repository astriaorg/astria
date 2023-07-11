#!/bin/sh -x
celestia bridge auth admin \
  --node.store "$home_dir/bridge" \
  --keyring.accname validator > "$home_dir"/.admin_token
