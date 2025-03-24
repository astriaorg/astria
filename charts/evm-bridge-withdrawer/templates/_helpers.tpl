{{/*
Namepsace to deploy elements into.
*/}}
{{- define "evm-bridge-withdrawer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "evm-bridge-withdrawer.image" -}}
{{ .Values.images.evmBridgeWithdrawer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.evmBridgeWithdrawer.devTag }}{{ else }}{{ .Values.images.evmBridgeWithdrawer.tag }}{{ end }}
{{- end }}

{{/*
application name to deploy elements into.
*/}}
{{- define "evm-bridge-withdrawer.appName" -}}
evm-bridge-withdrawer-{{ .Values.config.assetName }}
{{- end }}

{{/*
Signer endpoints config string
*/}}
{{- define "evm-bridge-withdrawer.frostParticipantEndpoints" }}
{{- range $index, $element := $.Values.config.frostParticipantEndpoints }}
{{- if $index }},{{- end }}{{- $element }}
{{- end }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "evm-bridge-withdrawer.labels" -}}
{{ include "evm-bridge-withdrawer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "evm-bridge-withdrawer.selectorLabels" -}}
app: {{ include "evm-bridge-withdrawer.appName" . }}
{{- end }}
