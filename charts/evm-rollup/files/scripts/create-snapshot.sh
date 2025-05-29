#!/bin/sh

set -o errexit -o nounset

K8S_ROLLUP_APP_NAME="{{ include "rollup.appName" . }}"
K8S_NAMESPACE="{{ include "rollup.namespace" . }}"
K8S_STATEFULSET_NAME="{{ include "rollup.name" . }}-geth"
K8S_STATEFULSET_REPLICA_COUNT="{{ .Values.global.replicaCount }}"

SNAPSHOT_PATH="{{ tpl .Values.geth.snapshot.create.storage.path $ }}"
SNAPSHOT_FILE="{{ include "rollup.name" . }}-snapshot-$(date +%Y-%m-%d-%T).tar.gz"
RETENTION_COUNT="{{ .Values.geth.snapshot.create.retentionCount }}"

echo "🛑 Scaling down for safe snapshot creation..."

kubectl scale statefulset $K8S_STATEFULSET_NAME -n $K8S_NAMESPACE --replicas=0

echo "⏳ Waiting for pods to terminate..."
timeout=300
elapsed=0
while [ $elapsed -lt $timeout ]; do
    pod_count=$(kubectl get pods -l app.kubernetes.io/name=$K8S_STATEFULSET_NAME -n $K8S_NAMESPACE --no-headers 2>/dev/null | wc -l)
    if [ "$pod_count" -eq 0 ]; then
        echo "✅ All pods terminated successfully"
        break
    fi
    echo "Waiting for $pod_count pod(s) to terminate... (${elapsed}s elapsed)"
    sleep 5
    elapsed=$((elapsed + 5))
done

if [ $elapsed -ge $timeout ]; then
    echo "⚠️ Warning: Timeout waiting for pods to terminate, proceeding anyway..."
fi

echo "💿 Creating snapshot at $SNAPSHOT_FILE"

mkdir -p "$SNAPSHOT_PATH"
tar -zcvf $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  -C $data_dir/geth \
  --exclude='chaindata/LOCK' \
  --exclude='chaindata/ancient/chain/FLOCK' \
  chaindata

SNAPSHOT_SIZE=$(du -sh $SNAPSHOT_PATH/$SNAPSHOT_FILE | cut -f1)
echo "📦 Snapshot created successfully ($SNAPSHOT_SIZE)"

SNAPSHOT_CHECKSUM=$(sha256sum "$SNAPSHOT_PATH/$SNAPSHOT_FILE" | cut -d ' ' -f 1)
echo "🛡️ Snapshot checksum: $SNAPSHOT_CHECKSUM"

{{- if .Values.geth.snapshot.create.storage.upload.enabled }}
echo "⬆️ Uploading snapshot to {{ .Values.geth.snapshot.create.storage.upload.destination }}"
rclone copy -vv \
  $SNAPSHOT_PATH/$SNAPSHOT_FILE \
  {{ .Values.geth.snapshot.create.storage.upload.destination }}
{{- end }}

echo "🧹 Cleaning up old snapshots (keeping last $RETENTION_COUNT)"
cd "$SNAPSHOT_PATH"
ls -t snapshot-*.tar.gz 2>/dev/null | tail -n +$((RETENTION_COUNT + 1)) | xargs -r rm --

echo "📈 Scaling back up..."
kubectl scale statefulset $K8S_STATEFULSET_NAME -n $K8S_NAMESPACE --replicas=$K8S_STATEFULSET_REPLICA_COUNT

echo "⏳ Waiting for pod to be ready..."
timeout=300
elapsed=0
while [ $elapsed -lt $timeout ]; do
    ready_pods=$(kubectl get pods -l app.kubernetes.io/name=$K8S_STATEFULSET_NAME -n $K8S_NAMESPACE -o jsonpath='{.items[?(@.status.conditions[?(@.type=="Ready")].status=="True")].metadata.name}' 2>/dev/null | wc -w)
    if [ "$ready_pods" -ge 1 ]; then
        echo "✅ Pod is ready and running"
        break
    fi
    echo "Waiting for pod to be ready... (${elapsed}s elapsed)"
    sleep 10
    elapsed=$((elapsed + 10))
done

if [ $elapsed -ge $timeout ]; then
    echo "⚠️ Warning: Timeout waiting for pod to be ready"
    echo "📋 Current pod status:"
    kubectl get pods -l app.kubernetes.io/name=$K8S_STATEFULSET_NAME -n $NAMESPACE
fi

echo "Snapshot created successfully 🎉"
