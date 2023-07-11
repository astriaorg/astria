default:
  @just --list

create-cluster:
  kind create cluster --config ./kubernetes-ci/cluster-config.yml

deploy-ingress-controller:
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

perform-prepull:
  kubectl apply -f ./kubernetes-ci/prepull-daemon-set.yml

start-celestia-jsonrpc-test-deployment:
  kubectl apply -k crates/astria-celestia-jsonrpc-client/k8s/

wait-for-ingress-controller:
  kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s

wait-for-prepull:
  kubectl wait --for=condition=ready pod --selector=name=astria-test-prepull --timeout=600s

wait-for-celestia-jsonrpc-test-deployment:
  kubectl wait --namespace astria-celestia-jsonrpc-client-test deployment --for=condition=Available --selector=app.kubernetes.io/name=astria-celestia-jsonrpc-client-test --timeout=600s
