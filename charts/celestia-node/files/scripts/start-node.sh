#!/bin/bash

set -o errexit -o nounset -o pipefail
{{- $isCustomNetwork := eq .Values.config.network "custom" }}

{{- if $isCustomNetwork }}
export CELESTIA_CUSTOM=$CELESTIA_CUSTOM_TO_BE
{{- end }}

exec /bin/celestia {{ .Values.config.type }} start \
  --node.store /celestia \
  --rpc.skip-auth \
  {{- if not $isCustomNetwork }}
  --core.ip {{ .Values.config.coreIp }} \
  --core.grpc.port "{{ .Values.config.coreGrpcPort }}" \
  --p2p.network {{ .Values.config.network }}
  {{- end }}
