{{/*
Namepsace to deploy elements into.
*/}}
{{- define "rollup.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*  The name of the rollup */}}
{{- define "rollup.name" -}}
{{- tpl .Values.global.rollupName . }}
{{- end }}

{{- define "rollup.genesis-file" -}}
files/genesis/{{ include "rollup.type" . }}.genesis.json
{{- end -}}

{{- define "rollup.networkId" }}
{{- $rollupType := (include "rollup.type" . ) -}}
{{- if eq $rollupType "flame-mainnet" -}}253368190
{{- else if eq $rollupType "flame-testnet" -}}16604737732183
{{- else if eq $rollupType "flame-devnet" -}}912559
{{- else if eq $rollupType "forma-testnet" -}}984123
{{- else -}}{{ tpl .Values.genesis.chainId . }}
{{- end -}}
{{- end }}

{{- define "rollup.repos.geth" -}}
{{- $rollupType := (include "rollup.type" . ) -}}
{{- if or (eq $rollupType "custom") .Values.global.dev -}}{{ .Values.images.geth.repo }}
{{- else if hasPrefix "flame-" $rollupType -}}ghcr.io/astriaorg/astria-geth
{{- else if hasPrefix "forma-" $rollupType -}}ghcr.io/forma-dev/forma-geth
{{- end -}}
{{- end }}

{{- define "rollup.tags.geth" -}}
{{- $rollupType := (include "rollup.type" . ) -}}
{{- if or (eq $rollupType "custom") .Values.global.dev -}}{{ .Values.images.geth.tag }}
{{- else if eq $rollupType "flame-mainnet" -}}1.1.0
{{- else if eq $rollupType "flame-testnet" -}}1.1.0
{{- else if eq $rollupType "flame-devnet" -}}2.0.0-beta.1
{{- else if eq $rollupType "forma-testnet" -}}2.0.0-beta.1-forma-dev.2
{{- end -}}
{{- end }}

{{- define "rollup.tags.conductor" -}}
{{- $rollupType := (include "rollup.type" . ) -}}
{{- if or (eq $rollupType "custom") .Values.global.dev -}}{{ .Values.images.conductor.tag }}
{{- else if eq $rollupType "flame-mainnet" -}}1.1.0
{{- else if eq $rollupType "flame-testnet" -}}1.1.0
{{- else if eq $rollupType "flame-devnet" -}}2.0.0-rc.1
{{- else if eq $rollupType "forma-testnet" -}}sha-08fe3e6
{{- end -}}
{{- end }}


{{- define "rollup.type" -}}
{{- $rollupName := (include "rollup.name" . ) -}}
{{- if eq $rollupName "flame" -}}flame-mainnet
{{- else if eq $rollupName "flame-dawn-1" -}}flame-testnet
{{- else if eq $rollupName "flame-dusk-11"}}flame-devnet
{{- else if eq $rollupName "forma-sketchpad"}}forma-testnet
{{- else -}}custom
{{- end -}}
{{- end }}


{{/* verbosity based on log level */}}
{{- define "rollup.verbosity" -}}
{{- if eq . "silent" }}0
{{- else if eq . "error" }}1
{{- else if eq . "warn" }}2
{{- else if eq . "info" }}3
{{- else if eq . "debug" }}4
{{- else if eq . "trace" }}5
{{- end }}
{{- end }}

{{- define "rollup.moduleVerbosity" -}}
{{- range $module := .Values.geth.moduleLogLevels }}{{$module.module}}={{ include "rollup.verbosity" $module.level }},
{{- end }}
{{- end }}

{{/*
Expand the name of the chart.
*/}}
{{- define "rollup.appName" -}}
{{- default (include "rollup.name" .) | trunc 63 | trimSuffix "-" }}-astria-dev-cluster
{{- end }}

{{/*
Common labels
*/}}
{{- define "rollup.labels" -}}
{{ include "rollup.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "rollup.selectorLabels" -}}
app: {{ include "rollup.appName" . }}
{{- end }}

{{/*
The log level represented as a number
*/}}
{{- define "rollup.logLevelNum" -}}
{{- if eq .Values.config.logLevel "error" -}}
1
{{- else if eq .Values.config.logLevel "warn" -}}
2
{{- else if eq .Values.config.logLevel "info" -}}
3
{{- else if eq .Values.config.logLevel "debug" -}}
4
{{- else if eq .Values.config.logLevel "trace" -}}
5
{{- end }}
{{- end }}

{{/*
Full image paths for Astria built images
*/}}
{{- define "rollup.image" -}}
{{ include "rollup.repos.geth" . }}:{{ include "rollup.tags.geth" . }}
{{- end }}

{{- define "conductor.image" -}}
{{ .Values.images.conductor.repo }}:{{ include "rollup.tags.conductor" . }}
{{- end }}


{{/*
Return if ingress is stable.
*/}}
{{- define "rollup.ingress.isStable" -}}
{{- eq (include "rollup.ingress.apiVersion" .) "networking.k8s.io/v1" }}
{{- end }}

{{/*
Return if ingress supports ingressClassName.
*/}}
{{- define "rollup.ingress.supportsIngressClassName" -}}
{{- or (eq (include "rollup.ingress.isStable" .) "true") (and (eq (include "rollup.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return if ingress supports pathType.
*/}}
{{- define "rollup.ingress.supportsPathType" -}}
{{- or (eq (include "rollup.ingress.isStable" .) "true") (and (eq (include "rollup.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return the appropriate apiVersion for ingress.
*/}}
{{- define "rollup.ingress.apiVersion" -}}
{{- if and ($.Capabilities.APIVersions.Has "networking.k8s.io/v1") (semverCompare ">= 1.19-0" .Capabilities.KubeVersion.Version) }}
{{- print "networking.k8s.io/v1" }}
{{- else if $.Capabilities.APIVersions.Has "networking.k8s.io/v1beta1" }}
{{- print "networking.k8s.io/v1beta1" }}
{{- else }}
{{- print "extensions/v1beta1" }}
{{- end }}
{{- end }}

{{- define "rollup.gethHomeDir" -}}
/home/geth
{{- end }}

{{- define "rollup.gethDataDir" -}}
{{ include "rollup.gethHomeDir" . }}/{{ include "rollup.name" . }}
{{- end }}
