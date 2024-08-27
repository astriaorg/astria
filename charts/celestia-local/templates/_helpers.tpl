{{/*
Namepsace to deploy elements into.
*/}}
{{- define "celestia.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}


{{/*
Expand the name of the chart.
*/}}
{{- define "celestia.appName" -}}
{{- default (include "celestia.name" .) | trunc 63 | trimSuffix "-" }}-astria-dev-cluster
{{- end }}

{{/*
Common labels
*/}}
{{- define "celestia.labels" -}}
{{ include "celestia.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "celestia.selectorLabels" -}}
app: {{ include "celestia.appName" . }}
{{- end }}

{{/*
The log level represented as a number
*/}}
{{- define "celestia.logLevelNum" -}}
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
{{- define "celestia.image" -}}
{{ .Values.images.geth.repo }}:{{ if .Values.images.geth.overrideTag }}{{ .Values.images.geth.overrideTag }}{{ else }}{{ if .Values.global.dev }}{{ .Values.images.geth.devTag }}{{ else }}{{ .Values.images.geth.tag }}{{ end }}
{{- end }}
{{- end }}
{{- define "conductor.image" -}}
{{ .Values.images.conductor.repo }}:{{ if .Values.global.dev }}{{ .Values.images.conductor.devTag }}{{ else }}{{ .Values.images.conductor.tag }}{{ end }}
{{- end }}
{{- define "composer.image" -}}
{{ .Values.images.composer.repo }}:{{ if .Values.global.dev }}{{ .Values.images.composer.devTag }}{{ else }}{{ .Values.images.composer.tag }}{{ end }}
{{- end }}


{{/*
Return if ingress is stable.
*/}}
{{- define "celestia.ingress.isStable" -}}
{{- eq (include "celestia.ingress.apiVersion" .) "networking.k8s.io/v1" }}
{{- end }}

{{/*
Return if ingress supports ingressClassName.
*/}}
{{- define "celestia.ingress.supportsIngressClassName" -}}
{{- or (eq (include "celestia.ingress.isStable" .) "true") (and (eq (include "celestia.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return if ingress supports pathType.
*/}}
{{- define "celestia.ingress.supportsPathType" -}}
{{- or (eq (include "celestia.ingress.isStable" .) "true") (and (eq (include "celestia.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return the appropriate apiVersion for ingress.
*/}}
{{- define "celestia.ingress.apiVersion" -}}
{{- if and ($.Capabilities.APIVersions.Has "networking.k8s.io/v1") (semverCompare ">= 1.19-0" .Capabilities.KubeVersion.Version) }}
{{- print "networking.k8s.io/v1" }}
{{- else if $.Capabilities.APIVersions.Has "networking.k8s.io/v1beta1" }}
{{- print "networking.k8s.io/v1beta1" }}
{{- else }}
{{- print "extensions/v1beta1" }}
{{- end }}
{{- end }}

{{- define "celestia.gethHomeDir" -}}
/home/geth
{{- end }}

{{- define "celestia.gethDataDir" -}}
{{ include "celestia.gethHomeDir" . }}/{{ include "celestia.name" . }}
{{- end }}
