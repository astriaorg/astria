#!/bin/bash
set -o errexit -o nounset -o pipefail

function set_token() {
  # NOTE - this is a hack to give access to a token generated on startup to people with ssh access
  TOKEN=$(/bin/celestia "$CELESTIA_NODE_TYPE" auth "$CELESTIA_TOKEN_AUTH_LEVEL" --node.store "/celestia")

  mkdir -p /celestia/token
  echo "$TOKEN" > /celestia/token/token.key
}

if [ ! -f /celestia/token/token.key ]; then
  set_token
fi
