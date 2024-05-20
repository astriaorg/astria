#!/bin/sh

set -o errexit -o nounset

# Only need to configure cometbft data if not already initialized
if [ ! -d "/cometbft/data" ]; then
  cp -LR /data/ /cometbft/data
fi

# Don't replace the config directory if it already exists
if [ ! -d "/cometbft/config" ]; then
  cp -LR /config/ /cometbft/config
else
  cp /config/* /cometbft/config/
fi

chmod -R 0777 /cometbft
