#!/bin/bash

set -o errexit -o nounset -o pipefail
{{- $isCustomNetwork := eq .Values.config.network "custom" }}

{{- if .Values.config.tokenAuthLevel }}
function set_token() {
  # NOTE - this is a hack to get the token to the token-server directory.
  TOKEN=$(/bin/celestia {{ .Values.config.type }} auth {{ .Values.config.tokenAuthLevel }} --node.store "/celestia")

  # Busybox's httpd doesn't support url rewriting, so it's not simple to server another file.
  # To support an ingress rule path of `/`, we write the token to index.html, which httpd serves by default.
  mkdir -p /celestia/token-server
  echo "$TOKEN" > /celestia/token-server/index.html
}

if [ ! -f /celestia/token-server/index.html ]; then
  set_token
fi
{{- end }}

{{- if $isCustomNetwork }}
export CELESTIA_CUSTOM=$CELESTIA_CUSTOM_TO_BE
{{- end }}

exec /bin/celestia {{ .Values.config.type }} start \
  --node.store /celestia \
  {{- if not $isCustomNetwork }}
  --core.ip {{ .Values.config.coreIp }} \
  --core.grpc.port "{{ .Values.config.coreGrpcPort }}" \
  --p2p.network {{ .Values.config.network }}
  {{- end }}
