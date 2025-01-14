#!/bin/bash

set -o errexit -o nounset

# Install tar if not present
if ! command -v tar &> /dev/null; then
    echo "🔧 Installing tar..."
    yum install -q -y tar gzip
fi

rm -rf "$data_dir/geth"
mkdir -p "$data_dir/geth"

SNAPSHOT="{{ .Values.config.geth.snapshot.restore.url }}"

echo "⏳ Loading snapshot from $SNAPSHOT"

if [[ "$SNAPSHOT" == s3://* ]]; then
  echo "⬇️ Downloading snapshot from S3"
  aws s3 cp "$SNAPSHOT" "$data_dir/snapshots/snapshot-to-load.tar.gz"
elif [[ "$SNAPSHOT" == http://* ]] || [[ "$SNAPSHOT" == https://* ]]; then
  if ! command -v curl &> /dev/null; then
    echo "🔧 Installing curl..."
    yum install -q -y curl
  fi
  echo "⬇️ Downloading snapshot from $SNAPSHOT"
  curl -fsSL $SNAPSHOT -o "$data_dir/snapshots/snapshot-to-load.tar.gz"
elif [[ "$SNAPSHOT" == file://* ]]; then
  echo "💿 Copying snapshot from $SNAPSHOT"
  cp "$SNAPSHOT" "$data_dir/snapshots/snapshot-to-load.tar.gz"
else
  echo "🚨 Invalid snapshot URL: $SNAPSHOT"
  exit 1
fi

{{if .Values.config.geth.snapshot.restore.checksum -}}
echo "🕵️ Verifying snapshot checksum..."
EXPECTED_CHECKSUM="{{ .Values.config.geth.snapshot.restore.checksum }}"
ACTUAL_CHECKSUM=$(sha256sum "$data_dir/snapshots/snapshot-to-load.tar.gz" | cut -d ' ' -f 1)

if [ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]; then
  echo "🚨 Checksum verification failed!"
  echo "Expected: $EXPECTED_CHECKSUM"
  echo "Got: $ACTUAL_CHECKSUM"
  exit 1
fi
echo "✅ Checksum verified successfully"
{{- end -}}

echo "Extracting snapshot..."
tar -xvf $data_dir/snapshots/snapshot-to-load.tar.gz -C $data_dir/geth

echo "🧹 Cleaning up..."
rm -f $data_dir/snapshots/snapshot-to-load.tar.gz

echo "Snapshot loaded successfully 🎉"
