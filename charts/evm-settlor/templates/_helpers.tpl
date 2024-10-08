{{/*
Namepsace to deploy elements into.
*/}}
{{- define "evm-settlor.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "evm-settlor.image" -}}
{{ .Values.images.evmBridgeWithdrawer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.evmBridgeWithdrawer.devTag }}{{ else }}{{ .Values.images.evmBridgeWithdrawer.tag }}{{ end }}
{{- end }}
