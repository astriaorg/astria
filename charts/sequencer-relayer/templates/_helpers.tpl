{{/*
Namepsace to deploy elements into.
*/}}
{{- define "sequencer-relayer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "sequencer-relayer.image" -}}
{{ .Values.images.sequencerRelayer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.sequencerRelayer.devTag }}{{ else }}{{ .Values.images.sequencerRelayer.tag }}{{ end }}
{{- end }}

{{/*
Expand the name of the chart.
*/}}
{{- define "sequencer-relayer.name" -}}
sequencer-relayer
{{- end }}

{{/*
Common labels
*/}}
{{- define "sequencer-relayer.labels" -}}
{{ include "sequencer-relayer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "sequencer-relayer.selectorLabels" -}}
app: {{ include "sequencer-relayer.name" . }}
{{- end }}

{{- define "sequencer-relayer.storage.mountPath" -}}
/astria-sequencer-relayer
{{- end }}

{{- define "sequencer-relayer.storage.submissionStatePath" -}}
{{ include "sequencer-relayer.storage.mountPath" . }}/submission-state.json
{{- end }}
