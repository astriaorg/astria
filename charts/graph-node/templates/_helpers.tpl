{{- define "graphnode.name" -}}
{{ .Release.Name }}
{{- end }}

{{- define "graphnode.fullname" -}}
{{ include "graphnode.name" . }}-graph-node
{{- end }}

{{/*
Namepsace to deploy elements into.
*/}}
{{- define "graphnode.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}
