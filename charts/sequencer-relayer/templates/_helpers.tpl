{{/*
Namepsace to deploy elements into.
*/}}
{{- define "sequencer-relayer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "sequencer-relayer.image" -}}
{{ .Values.images.sequencerRelayer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.sequencerRelayer.devTag }}{{ else }}{{ .Values.images.sequencerRelayer.tag }}{{ end }}
{{- end }}

{{- define "sequencer-relayer.storage.mountPath" -}}
/astria-sequencer-relayer
{{- end }}

{{- define "sequencer-relayer.storage.preSubmitPath" -}}
{{ include "sequencer-relayer.storage.mountPath" . }}/presubmit.json
{{- end }}

{{- define "sequencer-relayer.storage.postSubmitPath" -}}
{{ include "sequencer-relayer.storage.mountPath" . }}/postsubmit.json
{{- end }}
