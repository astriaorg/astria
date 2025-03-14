{{/*
Namepsace to deploy elements into.
*/}}
{{- define "sequencer-relayer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "sequencer-relayer.imageTag" -}}
{{- if or (eq .Values.global.network "custom") (eq .Values.global.dev true) }}{{ .Values.images.sequencerRelayer.tag }}
{{- else if eq .Values.global.network "mainnet" }}1.0.0
{{- else if eq .Values.global.network "dawn-1" }}1.0.0
{{- else if eq .Values.global.network "dusk-11" }}1.0.0
{{- end }}
{{- end }}

{{- define "sequencer-relayer.image" -}}
{{ .Values.images.sequencerRelayer.repo }}:{{ include "sequencer-relayer.imageTag" . }}
{{- end }}

{{- define "sequencer-relayer.sequencerChainId" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.config.relayer.sequencerChainId }}
{{- else if eq .Values.global.network "mainnet" }}astria
{{- else if eq .Values.global.network "dawn-1" }}dawn-1
{{- else if eq .Values.global.network "dusk-11" }}astria-dusk-11
{{- end }}
{{- end }}

{{- define "sequencer-relayer.celestiaChainId" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.config.relayer.celestiaChainId }}
{{- else if eq .Values.global.network "mainnet" }}celestia
{{- else if eq .Values.global.network "dawn-1" }}mocha-4
{{- else if eq .Values.global.network "dusk-11" }}mocha-4
{{- end }}
{{- end }}

{{- define "sequencer-relayer.blockTimeMs" -}}
{{- if or (eq .Values.global.network "custom") (eq .Values.global.dev true) }}{{ .Values.config.relayer.blockTimeMs }}
{{- else if eq .Values.global.network "mainnet" }}1000
{{- else if eq .Values.global.network "dawn-1" }}1000
{{- else if eq .Values.global.network "dusk-11" }}1000
{{- end }}
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
name: {{ include "sequencer-relayer.name" . }}-metrics
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

{{- define "sequencer-relayer.storage.submissionStatePath" -}}
{{ include "sequencer-relayer.storage.mountPath" . }}/submission-state.json
{{- end }}
