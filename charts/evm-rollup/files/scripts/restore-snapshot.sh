#!/bin/sh

set -o errexit -o nounset

rm -rf "$data_dir/geth"
mkdir -p "$data_dir/geth"

{{if .Values.config.geth.snapshot.restore.url -}}
SNAPSHOT="{{ .Values.config.geth.snapshot.restore.url }}"
{{- else -}}
SNAPSHOT="{{ .Values.config.geth.snapshot.restore.path }}"
{{- end}}

echo "Loading snapshot from $SNAPSHOT"

{{if .Values.config.geth.snapshot.restore.url -}}
echo "Downloading snapshot from $SNAPSHOT"
curl -L "$SNAPSHOT" -o "$data_dir/snapshots/snapshot-to-load.tar.gz"
{{- else -}}
echo "Copying snapshot from $SNAPSHOT"
cp "$SNAPSHOT" "$data_dir/snapshots/snapshot-to-load.tar.gz"
{{- end}}

{{- if .Values.config.geth.snapshot.restore.checksum -}}
echo "Verifying snapshot checksum..."
EXPECTED_CHECKSUM="{{ .Values.config.geth.snapshot.restore.checksum }}"
ACTUAL_CHECKSUM=$(sha256sum "$data_dir/snapshots/snapshot-to-load.tar.gz" | cut -d ' ' -f 1)

if [ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]; then
  echo "Checksum verification failed!"
  echo "Expected: $EXPECTED_CHECKSUM"
  echo "Got: $ACTUAL_CHECKSUM"
  exit 1
fi
echo "Checksum verified successfully"
{{- end}}

echo "Extracting snapshot..."
tar -xvf $data_dir/snapshots/snapshot-to-load.tar.gz -C $data_dir/geth

echo "Cleaning up..."
rm -f $data_dir/snapshots/snapshot-to-load.tar.gz

echo "Snapshot loaded successfully"
