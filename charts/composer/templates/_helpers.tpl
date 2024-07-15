{{/*
Namepsace to deploy elements into.
*/}}
{{- define "composer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
Single entry of rollup names
*/}}
{{- define "composer.rollupDefinition" }}
{{ .name }}::{{ .wsRpc }}
{{- end}}

{{/*
Rollup config string
*/}}
{{- define "composer.rollups" }}
{{- range $index, $element := .Values.config.rollups }}
{{- if $index }},{{- end }}{{- tpl (include "composer.rollupDefinition" $element) $ }}
{{- end }}
{{- end }}

{{- define "composer.image" -}}
{{ .Values.images.composer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.composer.devTag }}{{ else }}{{ .Values.images.composer.tag }}{{ end }}
{{- end }}
