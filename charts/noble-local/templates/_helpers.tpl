{{/*
Namepsace to deploy elements into.
*/}}
{{- define "noble-local.namespace" -}}
{{- default .Release.Namespace .Values.global.namespace | trunc 63 | trimSuffix "-" -}}
{{- end }}
