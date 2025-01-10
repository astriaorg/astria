#!/bin/bash

set -o errexit -o nounset

SNAPSHOT_PATH="{{ .Values.config.geth.snapshot.create.storage.path }}/snapshot-$(date +%Y-%m-%d-%T).tar.gz"

echo "💿 Creating snapshot at $SNAPSHOT_PATH"

mkdir -p "{{ .Values.config.geth.snapshot.create.storage.path }}"
tar -zcvf $SNAPSHOT_PATH \
  -C $data_dir/geth \
  --exclude='chaindata/LOCK' \
  --exclude='chaindata/ancient/chain/FLOCK' \
  chaindata

echo "📦 Snapshot created successfully"

{{if .Values.config.geth.snapshot.create.storage.s3.enabled -}}
echo "⬆️ Uploading snapshot to S3"
aws configure set region {{ .Values.aws.config.region }}
aws s3 cp $SNAPSHOT_PATH s3://{{ .Values.config.geth.snapshot.create.storage.s3.bucket }}/
{{- end}}

echo "Done 🎉"
