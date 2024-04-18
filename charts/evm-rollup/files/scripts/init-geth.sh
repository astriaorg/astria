#!/bin/bash

set -o errexit -o nounset

if [ ! -d $data_dir ] ; then
  mkdir -p $data_dir
fi

if [ -z "$(ls -A $data_dir/)" ]; then
  echo "Initializing geth db..."

  exec geth --datadir "$data_dir/" --db.engine {{ .Values.config.rollup.dbEngine }} \
    {{ if not .Values.config.rollup.archiveNode }}--state.scheme=path {{- end }} \
    init /scripts/geth-genesis.json
fi
