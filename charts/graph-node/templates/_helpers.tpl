{{/*
Namepsace to deploy elements into.
*/}}
{{- define "graph-node.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*  The name of the graph-node */}}
{{- define "graph-node.name" -}}
"{{ default "graph-node" .Values.config.graphNodeName }}"
{{- end }}

{{/*
Expand the name of the chart.
*/}}
{{- define "graph-node.appName" -}}
{{- default (include "graph-node.name" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "graph-node.labels" -}}
{{ include "graph-node.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "graph-node.selectorLabels" -}}
app: {{ include "graph-node.appName" . }}
{{- end }}

{{/*
Full image paths for Astria built images
*/}}
{{- define "graph-node.image" -}}
{{ .Values.images.graph-node.repo }}:{{ .Values.images.graph-node.tag }}
{{- end }}


{{/*
Return if ingress is stable.
*/}}
{{- define "graph-node.ingress.isStable" -}}
{{- eq (include "graph-node.ingress.apiVersion" .) "networking.k8s.io/v1" }}
{{- end }}

{{/*
Return if ingress supports ingressClassName.
*/}}
{{- define "graph-node.ingress.supportsIngressClassName" -}}
{{- or (eq (include "graph-node.ingress.isStable" .) "true") (and (eq (include "graph-node.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return if ingress supports pathType.
*/}}
{{- define "graph-node.ingress.supportsPathType" -}}
{{- or (eq (include "graph-node.ingress.isStable" .) "true") (and (eq (include "graph-node.ingress.apiVersion" .) "networking.k8s.io/v1beta1") (semverCompare ">= 1.18-0" .Capabilities.KubeVersion.Version)) }}
{{- end }}

{{/*
Return the appropriate apiVersion for ingress.
*/}}
{{- define "graph-node.ingress.apiVersion" -}}
{{- if and ($.Capabilities.APIVersions.Has "networking.k8s.io/v1") (semverCompare ">= 1.19-0" .Capabilities.KubeVersion.Version) }}
{{- print "networking.k8s.io/v1" }}
{{- else if $.Capabilities.APIVersions.Has "networking.k8s.io/v1beta1" }}
{{- print "networking.k8s.io/v1beta1" }}
{{- else }}
{{- print "extensions/v1beta1" }}
{{- end }}
{{- end }}

