_default:
  @just --list argo


# Create or update an ArgoCD ApplicationSet from dev/argocd/pr-preview-envs/{{appset}}-appset.yaml
appset-update appset:
  argocd appset create dev/argocd/pr-preview-envs/{{appset}}-appset.yaml --upsert
