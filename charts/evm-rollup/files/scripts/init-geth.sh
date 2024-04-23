#!/bin/bash

set -o errexit -o nounset

if [ ! -d "$data_dir/" ]; then
  echo "Initializing geth db..."

  cp /scripts/geth-genesis.json $home_dir/genesis.json

  exec geth --datadir "$data_dir/" --db.engine {{ .Values.config.rollup.dbEngine }} \
    {{ if not .Values.config.rollup.archiveNode }}--state.scheme=path {{- end }} \
    init $home_dir/genesis.json
elif ! cmp -s "/scripts/geth-genesis.json" "$home_dir/genesis.json"; then
  echo "Geth DB already initialized, but genesis file upgraded..."

  cp /scripts/geth-genesis.json $home_dir/genesis.json

  exec geth --datadir "$data_dir/" init $home_dir/genesis.json
fi
