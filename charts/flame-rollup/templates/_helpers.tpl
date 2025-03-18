{{/*
Namepsace to deploy elements into.
*/}}
{{- define "rollup.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*  The name of the rollup */}}
{{- define "rollup.name" -}}
{{- tpl .Values.genesis.rollupName . }}
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
{{- if eq .Values.config.logLevel "error" }}
1
{{- else if eq .Values.config.logLevel "warn" }}
2
{{- else if eq .Values.config.logLevel "info" }}
3
{{- else if eq .Values.config.logLevel "debug" }}
4
{{- else if eq .Values.config.logLevel "trace" }}
5
{{- end }}
{{- end }}

{{/*
Full image paths for Astria built images
*/}}
{{- define "rollup.image" -}}
{{ .Values.images.geth.repo }}:{{ if .Values.global.dev }}{{ .Values.images.geth.devTag }}{{ else }}{{ .Values.images.geth.tag }}{{ end }}
{{- end }}
{{- define "conductor.image" -}}
{{ .Values.images.conductor.repo }}:{{ if .Values.global.dev }}{{ .Values.images.conductor.devTag }}{{ else }}{{ .Values.images.conductor.tag }}{{ end }}
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
