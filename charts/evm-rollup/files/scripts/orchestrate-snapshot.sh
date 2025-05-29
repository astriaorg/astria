#!/bin/sh

set -o errexit -o nounset

ROLLUP_NAME="{{ include "rollup.name" . }}"
NAMESPACE="{{ include "rollup.namespace" . }}"
STATEFULSET_NAME="${ROLLUP_NAME}-geth"
REPLICA_COUNT="{{ .Values.global.replicaCount }}"
TEMPLATE_CRONJOB="${ROLLUP_NAME}-geth-snapshot-template"

echo "🚀 Starting snapshot orchestration for $ROLLUP_NAME"

echo "🛑 Scaling down StatefulSet..."
kubectl scale statefulset $STATEFULSET_NAME --replicas=0 -n $NAMESPACE

echo "⏳ Waiting for StatefulSet pod to terminate..."
kubectl wait --for=delete pod/${STATEFULSET_NAME}-0 -n $NAMESPACE --timeout=300s

# Verify no pods are running
pod_count=$(kubectl get pods -l app.kubernetes.io/name=$STATEFULSET_NAME -n $NAMESPACE --no-headers 2>/dev/null | wc -l)
if [ "$pod_count" -ne 0 ]; then
  echo "⚠️ Warning: $pod_count pod(s) still running, waiting additional 30s..."
  sleep 30
fi

echo "✅ StatefulSet scaled down, volume released"

echo "📸 Creating snapshot job from template..."
SNAPSHOT_JOB_NAME="${ROLLUP_NAME}-geth-snapshot-$(date +%Y%m%d%H%M%S)"

kubectl create job --from=cronjob/$TEMPLATE_CRONJOB $SNAPSHOT_JOB_NAME -n $NAMESPACE

echo "⏳ Waiting for snapshot job to complete..."
kubectl wait --for=condition=complete job/$SNAPSHOT_JOB_NAME -n $NAMESPACE --timeout=3600s

if [ $? -eq 0 ]; then
  echo "✅ Snapshot job completed successfully"
  echo "📋 Snapshot job logs:"
  kubectl logs job/$SNAPSHOT_JOB_NAME -n $NAMESPACE | grep -v "^chaindata/"
else
  echo "❌ Snapshot job failed or timed out"
  kubectl describe job/$SNAPSHOT_JOB_NAME -n $NAMESPACE
  kubectl logs job/$SNAPSHOT_JOB_NAME -n $NAMESPACE
fi

echo "📈 Scaling StatefulSet back up..."
kubectl scale statefulset $STATEFULSET_NAME --replicas=$REPLICA_COUNT -n $NAMESPACE

echo "⏳ Waiting for StatefulSet to be ready..."
kubectl wait --for=jsonpath='{.status.readyReplicas}'=$REPLICA_COUNT statefulset/$STATEFULSET_NAME -n $NAMESPACE --timeout=300s

if [ $? -eq 0 ]; then
  echo "Rollup is back online and ready! 🎉"
else
  echo "⚠️ Warning: StatefulSet may not be ready yet, check status manually"
  kubectl get statefulset $STATEFULSET_NAME -n $NAMESPACE
  kubectl get pods -l app.kubernetes.io/name=$STATEFULSET_NAME -n $NAMESPACE
fi

echo "Snapshot completed successfully! 🎉"
