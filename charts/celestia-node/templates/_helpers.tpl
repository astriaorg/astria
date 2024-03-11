{{/*
Namepsace to deploy elements into.
*/}}
{{- define "celestiaNode.namespace" -}}
{{- default .Release.Namespace .Values.global.namespaceOverride | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
Define the base label
*/}}
{{- define "celestiaNode.baseLabel" -}}
{{- if .Values.config.labelPrefix }}{{ .Values.config.labelPrefix }}-{{- end }}{{ .Values.config.name }}-{{ .Values.config.type }}-{{ .Values.config.chainId }}
{{- end }}

{{/*
Define the service name
*/}}
{{- define "celestiaNode.service.name" -}}
{{ include "celestiaNode.baseLabel" . }}-service
{{- end }}

{{/*
Define the k8s path to rpc service
*/}}
{{- define "celestiaNode.service.addresses.base" -}}
{{ include "celestiaNode.service.name" . }}.{{ .Values.global.namespace }}.svc.cluster.local:{{ .Values.ports.celestia.rpc }}
{{- end }}


{{/*
Define the k8s path to rpc service as http rpc
*/}}
{{- define "celestiaNode.service.addresses.rpc" -}}
http://{{ include "celestiaNode.service.addresses.base" . }}
{{- end }}

{{/*
{{- end }}

{{/*
Define the k8s path to rpc service as ws
*/}}
{{- define "celestiaNode.service.addresses.ws" -}}
ws://{{ include "celestiaNode.service.addresses.base" . }}
{{- end }}

{{/*
Define the token service name
*/}}
{{- define "celestiaNode.service.token.name" -}}
{{ include "celestiaNode.baseLabel" . }}-token-service
{{- end }}
{{/*
Define the k8s path to token service
*/}}
{{- define "celestiaNode.service.addresses.token" -}}
http://{{ include "celestiaNode.service.token.name" . }}.{{ .Values.global.namespace }}.svc.cluster.local:{{ .Values.ports.tokenServer }}
{{- end }}


{{/*
Is this a custom network?
*/}}
{{- define "celestiaNode.customNetwork" -}}
{{ eq .Values.config.network "custom" }}
{{- end }}

{{- define "test" -}}
{{ print include "celestiaNode.customNetwork" .}}
{{- end }}
