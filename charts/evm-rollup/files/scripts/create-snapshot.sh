#!/bin/bash

set -o errexit -o nounset

# Install tar if not present
if ! command -v tar &> /dev/null; then
    echo "🔧 Installing tar..."
    yum install -q -y tar gzip
fi

SNAPSHOT_PATH="{{ tpl .Values.config.geth.snapshot.create.storage.path $ }}"
SNAPSHOT_FILE="snapshot-$(date +%Y-%m-%d-%T).tar.gz"
RETENTION_COUNT="{{ .Values.config.geth.snapshot.create.retentionCount }}"

echo "💿 Creating snapshot at $SNAPSHOT_FILE"

mkdir -p "$SNAPSHOT_PATH"
tar -zcvf $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  -C $data_dir/geth \
  --exclude='chaindata/LOCK' \
  --exclude='chaindata/ancient/chain/FLOCK' \
  chaindata

echo "📦 Snapshot created successfully"

{{- if .Values.config.geth.snapshot.create.storage.s3.enabled}}
echo "⬆️ Uploading snapshot to S3"
aws s3 cp \
  --region {{ .Values.aws.config.region }} \
  --checksum-algorithm SHA256 \
  $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  s3://{{ .Values.config.geth.snapshot.create.storage.s3.bucket }}/{{ include "rollup.name" . }}-$SNAPSHOT_FILE
{{- end}}

echo "🧹 Cleaning up old snapshots (keeping last $RETENTION_COUNT)"
cd "$SNAPSHOT_PATH"
ls -t snapshot-*.tar.gz 2>/dev/null | tail -n +$((RETENTION_COUNT + 1)) | xargs -r rm --

echo "Done 🎉"
