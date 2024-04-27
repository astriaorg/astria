#!/bin/bash

set -o errexit -o nounset -o pipefail
{{- $isCustomNetwork := eq .Values.config.network "custom" }}

{{- if .Values.config.tokenAuthLevel }}
function set_token() {
  # NOTE - this is a hack to give access to a token generated on startup to people with ssh access
  TOKEN=$(/bin/celestia {{ .Values.config.type }} auth {{ .Values.config.tokenAuthLevel }} --node.store "/celestia")

  mkdir -p /celestia/token
  echo "$TOKEN" > /celestia/token/token.key
}

if [ ! -f /celestia/token/token.key ]; then
  set_token
fi
{{- end }}

{{- if $isCustomNetwork }}
export CELESTIA_CUSTOM=$CELESTIA_CUSTOM_TO_BE
{{- end }}

exec /bin/celestia {{ .Values.config.type }} start \
  --node.store /celestia \
  {{- if not .Values.config.tokenAuthLevel }}
  --rpc.skipAuth \
  {{- end }}
  {{- if not $isCustomNetwork }}
  --core.ip {{ .Values.config.coreIp }} \
  --core.grpc.port "{{ .Values.config.coreGrpcPort }}" \
  --p2p.network {{ .Values.config.network }}
  {{- end }}
