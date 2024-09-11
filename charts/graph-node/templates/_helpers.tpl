{{- define "graph-node.name" -}}
{{ .Release.Name }}
{{- end }}

{{- define "graph-node.fullname" -}}
{{ include "graph-node.name" . }}-graph-node
{{- end }}
