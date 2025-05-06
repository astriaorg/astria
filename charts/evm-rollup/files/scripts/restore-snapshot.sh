#!/bin/sh

set -o errexit -o nounset

rm -rf "$data_dir/geth"
mkdir -p "$data_dir/geth"

SNAPSHOT="{{ .Values.geth.snapshot.restore.source }}"
SNAPSHOT_FILE=$(basename "$SNAPSHOT")

echo "💿 Copying snapshot from $SNAPSHOT"

rclone copy -vv \
  {{ .Values.geth.snapshot.restore.source }} \
  "$data_dir/snapshot-load/"

{{if .Values.geth.snapshot.restore.checksum -}}
echo "🕵️ Verifying snapshot checksum..."
EXPECTED_CHECKSUM="{{ .Values.geth.snapshot.restore.checksum }}"
ACTUAL_CHECKSUM=$(sha256sum "$data_dir/snapshot-load/$SNAPSHOT_FILE" | cut -d ' ' -f 1)

if [ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]; then
  echo "🚨 Checksum verification failed!"
  echo "Expected: $EXPECTED_CHECKSUM"
  echo "Got: $ACTUAL_CHECKSUM"
  exit 1
fi
echo "✅ Checksum verified successfully"
{{- end}}

echo "Extracting snapshot..."
tar -xvf $data_dir/snapshot-load/$SNAPSHOT_FILE -C $data_dir/geth

echo "🧹 Cleaning up..."
rm -rf $data_dir/snapshot-load

echo "Snapshot loaded successfully 🎉"
