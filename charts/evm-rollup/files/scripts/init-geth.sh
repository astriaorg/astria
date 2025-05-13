#!/bin/bash

set -o errexit -o nounset

if [ ! -d "$data_dir/" ]; then
  echo "Initializing geth db..."

  cp /scripts/geth-genesis.json $home_dir/genesis.json

  exec geth --networkid={{ include "rollup.networkId" . }} \
    {{- range $name, $arg := .Values.geth.flags -}}
    {{- $noCondition := not (hasKey $arg "condition") }}
    {{- if or ($noCondition) (eq (tpl $arg.condition $) "true") }}
    --{{ $name }}{{ if $arg.value }}={{ tpl $arg.value $ }}{{ end }} \
    {{- end }}
    {{- end }}
    init $home_dir/genesis.json
elif ! cmp -s "/scripts/geth-genesis.json" "$home_dir/genesis.json"; then
  echo "Geth DB already initialized, but genesis file upgraded..."

  cp /scripts/geth-genesis.json $home_dir/genesis.json

  exec geth --datadir "$data_dir/" init $home_dir/genesis.json
elif [ "{{ .Values.geth.snapshot.restore.enabled }}" = "true" ]; then
  echo "Snapshot restore enabled, running geth init..."

  exec geth --datadir "$data_dir/" init $home_dir/genesis.json
fi

{{if .Values.geth.configToml -}}
# copy config to home dir
cp -f /scripts/config.toml $home_dir/config.toml
{{- end }}

echo "Geth initialized"

NODEKEY=$(cat $data_dir/geth/nodekey)
echo "Nodekey: $NODEKEY"
