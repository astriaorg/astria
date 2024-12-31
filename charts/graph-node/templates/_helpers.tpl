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

{{/*
Service annotations
*/}}
{{- define "graphNode.serviceAnnotations" }}
{{- if .Values.graphNode.additionalAnnotations }}
{{ toYaml .Values.graphNode.additionalAnnotations }}
{{- end }}
{{- if .Values.graphNode.service.annotations }}
{{ toYaml .Values.graphNode.service.annotations }}
{{- end }}
{{- end }}

{{- define "ipfs.serviceAnnotations" }}
{{- if .Values.ipfs.additionalAnnotations }}
{{ toYaml .Values.ipfs.additionalAnnotations }}
{{- end }}
{{- if .Values.ipfs.service.annotations }}
{{ toYaml .Values.ipfs.service.annotations }}
{{- end }}
{{- end }}
