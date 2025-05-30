#!/bin/sh

set -o errexit -o nounset

SNAPSHOT_PATH="{{ tpl .Values.geth.snapshot.create.storage.path $ }}"
SNAPSHOT_FILE="{{ include "rollup.name" . }}-snapshot-$(date +%Y-%m-%d-%T).tar.gz"
RETENTION_COUNT="{{ .Values.geth.snapshot.create.retentionCount }}"

echo "💿 Creating snapshot at $SNAPSHOT_FILE"

mkdir -p "$SNAPSHOT_PATH"
tar -zcvf $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  -C $data_dir/geth \
  --exclude='chaindata/LOCK' \
  --exclude='chaindata/ancient/chain/FLOCK' \
  chaindata

SNAPSHOT_SIZE=$(du -sh $SNAPSHOT_PATH/$SNAPSHOT_FILE | cut -f1)
SNAPSHOT_CHECKSUM=$(sha256sum "$SNAPSHOT_PATH/$SNAPSHOT_FILE" | cut -d ' ' -f 1)

echo "📦 Snapshot packaged successfully"

{{- if .Values.geth.snapshot.create.storage.upload.enabled }}
echo "⬆️ Uploading snapshot to {{ .Values.geth.snapshot.create.storage.upload.destination }}"
rclone copy \
  $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  {{ .Values.geth.snapshot.create.storage.upload.destination }}
{{- end }}

echo "🧹 Cleaning up old snapshots (keeping last $RETENTION_COUNT)"
cd "$SNAPSHOT_PATH"
ls -t snapshot-*.tar.gz 2>/dev/null | tail -n +$((RETENTION_COUNT + 1)) | xargs -r rm --

echo "Snapshot created successfully 🎉"
echo "    → size: $SNAPSHOT_SIZE"
echo "    → checksum: $SNAPSHOT_CHECKSUM"
