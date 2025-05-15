#!/bin/sh

set -o errexit -o nounset

# Only need to configure cometbft data if not already initialized
if [ ! -d "/cometbft/data" ]; then
  # Load the snapshot on load if enabled
  {{- if .Values.snapshotLoad.enabled }}
  echo "Downdloading snapshot..."
  rclone config create r2 s3 \
    provider={{ .Values.snapshotLoad.config.provider }} \
    access_key_id={{ .Values.snapshotLoad.config.accessKeyId }} \
    secret_access_key={{ .Values.snapshotLoad.config.secretAccessKey }} \
    region={{ .Values.snapshotLoad.config.region }} \
    endpoint={{ .Values.snapshotLoad.config.endpoint }} \
    acl={{ .Values.snapshotLoad.config.acl }}
  rclone copy -P r2:astria-mainnet-snapshots/ /snapshot/

  echo "Extracting snapshot..."
  mkdir /cometbft/data
  mkdir /sequencer/penumbra.db
  tar -C /cometbft/data/ --strip-components=2 -xzf /snapshot/cometbft_*.tar.gz cometbft/data
  tar -C /sequencer/penumbra.db/ --strip-components=2 -xzf /snapshot/sequencer_*.tar.gz sequencer/penumbra.db
  rm /snapshot/cometbft_*.tar.gz /snapshot/sequencer_*.tar.gz
  {{- else }}
  # Otherwise initialize with basic values
  echo "Intializing cometbft with empty data directory..."
  cp -LR /data/ /cometbft/data
  {{- end }}
else
  echo "CometBFT data directory already initialized"
fi

# Don't replace the config directory if it already exists
if [ ! -d "/cometbft/config" ]; then
  echo "Creating Config Directory..."
  cp -LR /config/ /cometbft/config
else
  echo "Updating config directory..."
  cp /config/* /cometbft/config/
fi
