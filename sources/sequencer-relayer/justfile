default:
  @just --list

create-cluster:
  kind create cluster --config ./test_environment/cluster-config.yml

delete-cluster:
  kind delete cluster --name test-cluster

deploy-ingress-controller:
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

perform-prepull:
  kubectl apply -f ./test_environment/prepull-daemon-set.yml

prepare-test-environment: create-cluster deploy-ingress-controller perform-prepull

create-namespace:
  kubectl create namespace test

deploy-test-environment:
  kubectl apply -n test -k ./test_environment/

query-sequencer:
  curl http://test.localdev.me/sequencer/cosmos/base/tendermint/v1beta1/blocks/latest

wait-for-ingress-controller:
  kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s

wait-for-prepull:
  kubectl wait --for=condition=ready pod --selector=name=sequencer-relayer-environment-prepull --timeout=600s

wait-for-test-environment:
  kubectl wait -n test --for=condition=available deployment.apps/sequencer-relayer-environment-deployment --timeout=600s

kustomize:
  kubectl kustomize ./test_environment -o ./test_environment/test-environment.yml

create-ingress-rule:
  kubectl apply -n test -f ./test_environment/ingress.yml
