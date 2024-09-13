_default:
  @just --list argo

appset-update appset:
  argocd appset create dev/argocd/pr-preview-envs/{{appset}}-appset.yaml --upsert
