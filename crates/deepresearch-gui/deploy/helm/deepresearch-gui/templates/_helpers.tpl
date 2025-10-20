{{- define "deepresearch-gui.name" -}}
{{- default .Chart.Name .Values.nameOverride -}}
{{- end -}}

{{- define "deepresearch-gui.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name (include "deepresearch-gui.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
