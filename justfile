default:
  @just --list

create-cluster:
  kind create cluster --config ./crates/astria-celestia-jsonrpc-client/k8s/cluster-config.yml

deploy-ingress-controller:
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

wait-for-ingress-controller:
  kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s

start-celestia-jsonrpc-test-deployment:
  kubectl apply -k crates/astria-celestia-jsonrpc-client/k8s/

wait-for-celestia-jsonrpc-test-deployment:
  kubectl wait --namespace astria-celestia-jsonrpc-client-test deployment --for=condition=Available --selector=app.kubernetes.io/name=astria-celestia-jsonrpc-client-test --timeout=600s

delete-cluster:
  kind delete cluster --name astria-celestia-jsonrpc-client-test

default_docker_tag := 'local'

docker-build crate tag=default_docker_tag:
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{crate}}:{{tag}} .

fmt-rust:
  cargo +nightly-2023-08-18 fmt --all 

lint-rust:
  cargo +nightly-2023-08-18 fmt --all -- --check

fmt-toml:
  taplo format

lint-toml:
  taplo format --check

lint-md:
  markdownlint-cli2 "**/*.md" "#target" "#.github"
