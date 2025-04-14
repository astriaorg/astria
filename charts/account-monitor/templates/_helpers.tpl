{{/*
Namepsace to deploy elements into.
*/}}
{{- define "account-monitor.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
application name to deploy elements into.
*/}}
{{- define "account-monitor.appName" -}}
account-monitor
{{- end }}

{{/*
Common labels
*/}}
{{- define "account-monitor.labels" -}}
{{ include "account-monitor.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "account-monitor.selectorLabels" -}}
app: {{ include "account-monitor.appName" . }}
{{- end }}

{{- define "account-monitor.image" -}}
{{ .Values.images.accountMonitor.repo }}:{{ if .Values.global.dev }}{{ .Values.images.accountMonitor.devTag }}{{ else }}{{ .Values.images.accountMonitor.tag }}{{ end }}
{{- end }}
