{{/*
Namepsace to deploy elements into.
*/}}
{{- define "sequencer.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "sequencer.imageTag" -}}
{{- if or (eq .Values.global.network "custom") (eq .Values.global.dev true) }}{{ .Values.images.sequencer.tag }}
{{- else if eq .Values.global.network "mainnet" }}2.0.1
{{- else if eq .Values.global.network "dawn-1" }}2.0.1
{{- else if eq .Values.global.network "dusk-11" }}2.0.1
{{- end }}
{{- end }}

{{- define "cometBFT.imageTag" -}}
{{- if or (eq .Values.global.network "custom") (eq .Values.global.dev true) }}{{ .Values.images.cometBFT.tag }}
{{- else if eq .Values.global.network "mainnet" }}v0.38.17
{{- else if eq .Values.global.network "dawn-1" }}v0.38.17
{{- else if eq .Values.global.network "dusk-11" }}v0.38.17
{{- end }}
{{- end }}

{{- define "sequencer.image" -}}
{{ .Values.images.sequencer.repo }}:{{ include "sequencer.imageTag" . }}
{{- end }}
{{- define "cometBFT.image" -}}
{{ .Values.images.cometBFT.repo }}:{{ include "cometBFT.imageTag" . }}
{{- end }}
{{- define "priceFeed.image" -}}
{{ .Values.images.priceFeed.repo }}:{{ if .Values.global.dev }}{{ .Values.images.priceFeed.devTag }}{{ else }}{{ .Values.images.priceFeed.tag }}{{ end }}
{{- end }}

{{- define "cometBFT.timeouts.propose" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutPropose }}
{{- else if eq .Values.global.network "mainnet" }}2s
{{- else if eq .Values.global.network "dawn-1" }}2s
{{- else if eq .Values.global.network "dusk-11" }}2s
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.proposeDelta" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutProposeDelta }}
{{- else if eq .Values.global.network "mainnet" }}500ms
{{- else if eq .Values.global.network "dawn-1" }}500ms
{{- else if eq .Values.global.network "dusk-11" }}500ms
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.prevote" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutPrevote }}
{{- else if eq .Values.global.network "mainnet" }}1s
{{- else if eq .Values.global.network "dawn-1" }}1s
{{- else if eq .Values.global.network "dusk-11" }}1s
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.prevoteDelta" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutPrevoteDelta }}
{{- else if eq .Values.global.network "mainnet" }}500ms
{{- else if eq .Values.global.network "dawn-1" }}500ms
{{- else if eq .Values.global.network "dusk-11" }}500ms
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.precommit" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutPrecommit }}
{{- else if eq .Values.global.network "mainnet" }}1s
{{- else if eq .Values.global.network "dawn-1" }}1s
{{- else if eq .Values.global.network "dusk-11" }}1s
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.precommitDelta" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutPrecommitDelta }}
{{- else if eq .Values.global.network "mainnet" }}500ms
{{- else if eq .Values.global.network "dawn-1" }}500ms
{{- else if eq .Values.global.network "dusk-11" }}500ms
{{- end }}
{{- end }}

{{- define "cometBFT.timeouts.commit" -}}
{{- if eq .Values.global.network "custom" }}{{ .Values.cometbft.config.consensus.timeoutCommit }}
{{- else if eq .Values.global.network "mainnet" }}1500ms
{{- else if eq .Values.global.network "dawn-1" }}1500ms
{{- else if eq .Values.global.network "dusk-11" }}1500ms
{{- end }}
{{- end }}


{{/*
Return if ingress is stable.
*/}}
{{- define "sequencer.ingress.isStable" -}}
{{- eq (include "sequencer.ingress.apiVersion" .) "networking.k8s.io/v1" }}
{{- end }}

{{/*
Return if ingress supports ingressClassName.
*/}}
{{- define "sequencer.ingress.supportsIngressClassName" -}}
{{- or (eq (include "sequencer.ingress.isStable" .) "true") (and (eq (include "sequencer.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return if ingress supports pathType.
*/}}
{{- define "sequencer.ingress.supportsPathType" -}}
{{- or (eq (include "sequencer.ingress.isStable" .) "true") (and (eq (include "sequencer.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

Return the appropriate apiVersion for ingress.
*/}}
{{- define "sequencer.ingress.apiVersion" -}}
{{- if and ($.Capabilities.APIVersions.Has "networking.k8s.io/v1") (semverCompare ">= 1.19-0" .Capabilities.KubeVersion.Version) }}
{{- print "networking.k8s.io/v1" }}
{{- else if $.Capabilities.APIVersions.Has "networking.k8s.io/v1beta1" }}
{{- print "networking.k8s.io/v1beta1" }}
{{- else }}
{{- print "extensions/v1beta1" }}
{{- end }}
{{- end }}


{{/*
Expand the name of the chart.
*/}}
{{- define "sequencer.name" -}}
{{- default .Values.moniker | trunc 63 | trimSuffix "-" }}-sequencer
{{- end }}

{{/*
Common labels
*/}}
{{- define "sequencer.labels" -}}
{{ include "sequencer.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "sequencer.selectorLabels" -}}
app: {{ include "sequencer.name" . }}
name: {{ .Values.moniker }}-sequencer-metrics
{{- end }}

{{/* New sequencer address */}}
{{- define "sequencer.address" -}}
{ "bech32m": "{{ . }}" }
{{- end }}

{{/* uint64 fee converted to a astria proto Uint128 with only lo set */}}
{{- define "sequencer.toUint128Proto" -}}
{ "lo": {{ . }} }
{{- end }}

{{- define "sequencer.socket_directory" -}}
/sockets/
{{- end }}

{{- define "sequencer.abci_url" -}}
{{- if .Values.sequencer.abciUDS -}}
unix://{{- include "sequencer.socket_directory" . }}abci.sock
{{- else -}}
tcp://127.0.0.1:{{ .Values.ports.sequencerABCI }}
{{- end }}
{{- end }}
