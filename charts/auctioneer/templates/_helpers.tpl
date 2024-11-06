{{/*
Namepsace to deploy elements into.
*/}}
{{- define "auctioneer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
application name to deploy elements into.
*/}}
{{- define "auctioneer.appName" -}}
auctioneer
{{- end }}

{{/*
Common labels
*/}}
{{- define "auctioneer.labels" -}}
{{ include "auctioneer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "auctioneer.selectorLabels" -}}
app: {{ include "auctioneer.appName" . }}
{{- end }}

{{- define "auctioneer.image" -}}
{{ .Values.images.auctioneer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.auctioneer.devTag }}{{ else }}{{ .Values.images.auctioneer.tag }}{{ end }}
{{- end }}
