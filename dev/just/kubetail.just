# commands to simplify Kubetail usage

_default:
  @just --list kubetail

# add the kubetail chart repository
add-repo:
  helm repo add kubetail https://kubetail-org.github.io/helm-charts/

# install kubetail chart
install:
  # ensure the kubetail chart repository is added
  just kubetail add-repo
  helm install kubetail kubetail/kubetail --namespace kubetail --create-namespace

# uninstall kubetail chart
uninstall:
  helm uninstall -n kubetail kubetail

# port forward the kubetail service so we can connect on our local machine
port-forward:
  kubectl port-forward -n kubetail svc/kubetail 4000:80
