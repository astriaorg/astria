{{/*
Namepsace to deploy elements into.
*/}}
{{- define "bridge-signer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
application name to deploy elements into.
*/}}
{{- define "bridge-signer.appName" -}}
bridge-signer-{{ .Values.config.signerName }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "bridge-signer.labels" -}}
{{ include "bridge-signer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "bridge-signer.selectorLabels" -}}
app: {{ include "bridge-signer.appName" . }}
{{- end }}

{{- define "bridge-signer.image" -}}
{{ .Values.images.bridgeSigner.repo }}:{{ if .Values.global.dev }}{{ .Values.images.bridgeSigner.devTag }}{{ else }}{{ .Values.images.bridgeSigner.tag }}{{ end }}
{{- end }}
