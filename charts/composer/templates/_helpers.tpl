{{/*
Namepsace to deploy elements into.
*/}}
{{- define "composer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
application name to deploy elements into.
*/}}
{{- define "composer.appName" -}}
composer
{{- end }}

{{/*
Common labels
*/}}
{{- define "composer.labels" -}}
{{ include "composer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "composer.selectorLabels" -}}
app: {{ include "composer.appName" . }}
{{- end }}

{{/*
Single entry of rollup names
*/}}
{{- define "composer.rollupDefinition" }}
{{ .name }}::{{ .wsRpc }}
{{- end }}

{{- define "composer.rollupType" }}
{{- $rollupName := (include "composer.rollupName" . ) -}}
{{- if eq $rollupName "flame" -}}flame-mainnet
{{- else if eq $rollupName "flame-dawn-1" -}}flame-testnet
{{- else if eq $rollupName "flame-dusk-11"}}flame-devnet
{{- else -}}custom
{{- end -}}
{{- end }}

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
