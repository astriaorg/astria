#!/bin/bash

set -o errexit -o nounset

if [ -d "$($data_dir/)" ]; then
  echo "Initializing geth db..."

  exec geth --datadir "$data_dir/" --db.engine {{ .Values.config.rollup.dbEngine }} \
    {{ if not .Values.config.rollup.archiveNode }}--state.scheme=path {{- end }} \
    init /scripts/geth-genesis.json
elif [! $(cmp -s "/scripts/geth-genesis.json" "/genesis.json")]; then
  echo "Geth DB already initialized, but genesis file upgraded..."

  exec geth --datadir "$data_dir/" init /scripts/geth-genesis.json
fi
