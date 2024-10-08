{{/*
Namepsace to deploy elements into.
*/}}
{{- define "astrotrek.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "indexer.image" -}}
{{ .Values.images.indexer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.indexer.devTag }}{{ else }}{{ .Values.images.indexer.tag }}{{ end }}
{{- end }}

{{- define "api.image" -}}
{{ .Values.images.api.repo }}:{{ if .Values.global.dev }}{{ .Values.images.api.devTag }}{{ else }}{{ .Values.images.api.tag }}{{ end }}
{{- end }}

{{- define "frontend.image" -}}
{{ .Values.images.frontend.repo }}:{{ if .Values.global.dev }}{{ .Values.images.frontend.devTag }}{{ else }}{{ .Values.images.frontend.tag }}{{ end }}
{{- end }}
