default:
  @just --list

create-cluster:
  kind create cluster --config ./astria-conductor-test/kubernetes/cluster-config.yml

delete-cluster:
  kind delete cluster --name test-cluster

deploy-ingress-controller:
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

perform-prepull:
  kubectl apply -f ./astria-conductor-test/kubernetes/prepull-daemon-set.yml

prepare-test-environment: create-cluster deploy-ingress-controller perform-prepull

create-namespace:
  kubectl create namespace test

deploy-test-environment:
  kubectl apply -n test -k ./astria-conductor-test/kubernetes

query-sequencer:
  curl http://test.localdev.me/sequencer/cosmos/base/tendermint/v1beta1/blocks/latest

wait-for-ingress-controller:
  kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s

wait-for-prepull:
  kubectl wait --for=condition=ready pod --selector=name=conductor-environment-prepull --timeout=600s

wait-for-test-environment:
  kubectl wait -n test --for=condition=available deployment.apps/conductor-environment-deployment --timeout=600s

kustomize:
  kubectl kustomize ./astria-conductor-test/kubernetes -o ./astria-conductor-test/kubernetes/test-environment.yml

create-ingress-rule:
  kubectl apply -n test -f ./astria-conductor-test/kubernetes/ingress.yml
