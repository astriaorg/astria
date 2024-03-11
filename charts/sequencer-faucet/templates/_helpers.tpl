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