default:
  @just --list

default_docker_tag := 'local'

# Builds docker image for the crate. Defaults to 'local' tag.
docker-build crate tag=default_docker_tag:
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{crate}}:{{tag}} .

install-cli:
  cargo install --path ./crates/astria-cli --locked

# Compiles the generated rust code from protos which are used in crates.
compile-protos:
  cargo run --manifest-path tools/protobuf-compiler/Cargo.toml

####################################################
## Scripts related to formatting code and linting ##
####################################################

default_lang := 'all'

# Can format 'rust', 'toml', 'proto', or 'all'. Defaults to all
fmt lang=default_lang:
  @just _fmt-{{lang}}

# Can lint 'rust', 'toml', 'proto', 'md' or 'all'. Defaults to all.
lint lang=default_lang:
  @just _lint-{{lang}}

_fmt-all:
  @just _fmt-rust
  @just _fmt-toml
  @just _fmt-proto

@_lint-all:
  -just _lint-rust
  -just _lint-toml
  -just _lint-proto
  -just _lint-md

[no-exit-message]
_fmt-rust:
  cargo +nightly-2024-02-07 fmt --all

[no-exit-message]
_lint-rust:
  cargo +nightly-2024-02-07 fmt --all -- --check
  cargo clippy -- --warn clippy::pedantic
  cargo dylint --all

[no-exit-message]
_fmt-toml:
  taplo format

[no-exit-message]
_lint-toml:
  taplo format --check

[no-exit-message]
_lint-md:
  markdownlint-cli2 "**/*.md" "#target" "#.github"

[no-exit-message]
_fmt-proto:
  buf format -w

[no-exit-message]
_lint-proto:
  buf lint
  buf format -d --exit-code
  buf breaking proto/executionapis --against 'buf.build/astria/execution-apis'
  buf breaking proto/sequencerapis --against 'buf.build/astria/astria'
  buf breaking proto/composerapis --against 'buf.build/astria/composer-apis'

##############################################
## Deploying and Running using Helm and K8s ##
##############################################
defaultNamespace := "astria-dev-cluster"
deploy tool *ARGS:
  @just deploy-{{tool}} {{ARGS}}

delete tool *ARGS:
  @just delete-{{tool}} {{ARGS}}

load-image image:
  kind load docker-image {{image}} --name astria-dev-cluster

deploy-all: deploy-cluster deploy-ingress-controller wait-for-ingress-controller deploy-astria-local wait-for-sequencer (deploy-chart "sequencer-faucet") deploy-dev-rollup wait-for-rollup
delete-all: clean clean-persisted-data

deploy-astria-local namespace=defaultNamespace: (deploy-chart "celestia-local" namespace) (deploy-sequencer)
delete-astria-local namespace=defaultNamespace: (delete-chart "celestia-local" namespace) (delete-sequencer)

[private]
deploy-chart chart namespace=defaultNamespace:
  helm install {{chart}}-chart ./charts/{{chart}} --namespace {{namespace}} --create-namespace

[private]
delete-chart chart namespace=defaultNamespace:
  helm uninstall {{chart}}-chart --namespace {{namespace}}

[private]
helm-add-if-not-exist repo url:
  helm repo list | grep -q {{repo}} || helm repo add {{repo}} {{url}}

deploy-cluster namespace=defaultNamespace:
  kind create cluster --config ./dev/kubernetes/kind-cluster-config.yml
  @just helm-add-if-not-exist cilium https://helm.cilium.io/
  helm install cilium cilium/cilium --version 1.14.3 \
      -f ./dev/values/cilium.yml \
      --namespace kube-system
  kubectl create namespace {{namespace}}

deploy-ingress-controller:
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

[private]
deploy-celestia-local namespace=defaultNamespace: (deploy-chart "celestia-local" namespace)

[private]
delete-celestia-local namespace=defaultNamespace: (delete-chart "celestia-local" namespace)

deploy-secrets-store:
  @just helm-add-if-not-exist secrets-store-csi-driver https://kubernetes-sigs.github.io/secrets-store-csi-driver/charts
  helm install csi-secrets-store secrets-store-csi-driver/secrets-store-csi-driver --namespace kube-system

delete-secrets-store:
  @just delete chart csi-secrets-store kube-system

wait-for-ingress-controller:
  while ! kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s; do \
    sleep 1; \
  done

validatorName := "single"
deploy-sequencer name=validatorName:
  helm dependency build charts/sequencer > /dev/null
  helm install --debug \
    {{ replace('-f dev/values/validators/#.yml' , '#', name) }} \
    -n astria-validator-{{name}} --create-namespace \
    {{name}}-sequencer-chart ./charts/sequencer
deploy-sequencers: (deploy-sequencer "node0") (deploy-sequencer "node1") (deploy-sequencer "node2")

delete-sequencer name=validatorName:
  @just delete chart {{name}}-sequencer astria-validator-{{name}}
delete-sequencers: (delete-sequencer "node0") (delete-sequencer "node1") (delete-sequencer "node2")

wait-for-sequencer:
  kubectl wait -n astria-dev-cluster deployment celestia-local --for=condition=Available=True --timeout=600s
  kubectl rollout status --watch statefulset/sequencer -n astria-dev-cluster --timeout=600s

defaultRollupName          := "astria"
defaultNetworkId           := ""
defaultGenesisAllocAddress := ""
defaultPrivateKey          := ""
defaultSequencerStartBlock := ""
deploy-rollup rollupName=defaultRollupName networkId=defaultNetworkId genesisAllocAddress=defaultGenesisAllocAddress privateKey=defaultPrivateKey sequencerStartBlock=defaultSequencerStartBlock:
  helm dependency build charts/evm-rollup > /dev/null
  helm install \
    {{ if rollupName          != '' { replace('--set config.rollup.name=# --set celestia-node.config.labelPrefix=#', '#', rollupName) } else { '' } }} \
    {{ if networkId           != '' { replace('--set config.rollup.networkId=#', '#', networkId) } else { '' } }} \
    {{ if genesisAllocAddress != '' { replace('--set config.rollup.genesisAccounts[0].address=#', '#', genesisAllocAddress) } else { '' } }} \
    {{ if privateKey          != '' { replace('--set config.faucet.privateKey=#', '#', privateKey) } else { '' } }} \
    {{ if sequencerStartBlock != '' { replace('--set config.sequencer.initialBlockHeight=#', '#', sequencerStartBlock) } else { '' } }} \
    {{rollupName}}-chain-chart ./charts/evm-rollup --namespace astria-dev-cluster

deploy-dev-rollup rollupName=defaultRollupName networkId=defaultNetworkId genesisAllocAddress=defaultGenesisAllocAddress privateKey=defaultPrivateKey sequencerStartBlock=defaultSequencerStartBlock:
  helm dependency build charts/evm-rollup > /dev/null
  helm install \
    {{ if rollupName          != '' { replace('--set config.rollup.name=# --set celestia-node.config.labelPrefix=#', '#', rollupName) } else { '' } }} \
    {{ if networkId           != '' { replace('--set config.rollup.networkId=#', '#', networkId) } else { '' } }} \
    {{ if genesisAllocAddress != '' { replace('--set config.rollup.genesisAccounts[0].address=#', '#', genesisAllocAddress) } else { '' } }} \
    {{ if privateKey          != '' { replace('--set config.faucet.privateKey=#', '#', privateKey) } else { '' } }} \
    {{ if sequencerStartBlock != '' { replace('--set config.sequencer.initialBlockHeight=#', '#', sequencerStartBlock) } else { '' } }} \
    -f dev/values/rollup/dev.yaml \
    {{rollupName}}-chain-chart ./charts/evm-rollup --namespace astria-dev-cluster

delete-rollup rollupName=defaultRollupName:
  @just delete chart {{rollupName}}-chain

wait-for-rollup rollupName=defaultRollupName:
  kubectl rollout status --watch statefulset/{{rollupName}}-geth -n astria-dev-cluster --timeout=600s

defaultHypAgentConfig         := ""
defaultHypRelayerPrivateKey   := ""
defaultHypValidatorPrivateKey := ""
deploy-hyperlane-agents rollupName=defaultRollupName agentConfig=defaultHypAgentConfig relayerPrivateKey=defaultHypRelayerPrivateKey validatorPrivateKey=defaultHypValidatorPrivateKey:
  helm install --debug \
    {{ if rollupName          != '' { replace('--set config.name=# --set global.namespace=#-dev-cluster', '#', rollupName) } else { '' } }} \
    {{ if agentConfig         != '' { replace('--set config.agentConfig=#', '#', agentConfig) } else { '' } }} \
    {{ if relayerPrivateKey   != '' { replace('--set config.relayer.privateKey=#', '#', relayerPrivateKey) } else { '' } }} \
    {{ if validatorPrivateKey != '' { replace('--set config.validator.privateKey=#', '#', validatorPrivateKey) } else { '' } }} \
    {{rollupName}}-hyperlane-agents-chart ./charts/hyperlane-agents --namespace astria-dev-cluster

delete-hyperlane-agents rollupName=defaultRollupName:
  @just delete {{rollupName}}-hyperlane-agents

clean:
  kind delete cluster --name astria-dev-cluster

clean-persisted-data:
  rm -r /tmp/astria

deploy-local-metrics:
  kubectl apply -f kubernetes/metrics-server-local.yml

defaultTag := ""

deploy-smoke-test tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  @just wait-for-ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @helm dependency build charts/sequencer > /dev/null
  @helm dependency build charts/evm-rollup > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart ./charts/sequencer \
    -f dev/values/validators/single.yml \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @just wait-for-sequencer > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart ./charts/evm-rollup -f dev/values/rollup/dev.yaml \
    {{ if tag != '' { replace('--set images.conductor.devTag=# --set images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set config.blockscout.enabled=false \
    --set config.faucet.enabled=false > /dev/null
  @just wait-for-rollup > /dev/null
  @sleep 15

run-smoke-test:
  #!/usr/bin/env bash
  ETH_RPC_URL="http://executor.astria.localdev.me/"
  MAX_RUNS=30
  echo "Testing Transfer..."
  TRANSFER_RUNS=0
  EXPECTED_BALANCE=1000000000000000000
  curl -X POST $ETH_RPC_URL -s -d '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0xf86d80843c54e7f182520894830b0e9bb0b1ebad01f2805278ede64c69e068fe880de0b6b3a764000080820a96a045cac19cec50c92e356c665172ec70de5f3cd3721ba09bf3cbad1976d3e83487a00ff4d49607db9ac3c4bb71160be41600f8d1b56ac20b092c0e042f0d226e5277"],"id":1}' -H 'Content-Type: application/json' -s
  balance() {
    HEX_NUM=$(curl -X POST $ETH_RPC_URL -s -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0x830B0e9Bb0B1ebad01F2805278Ede64c69e068FE", "latest"],"id":1}' -H 'Content-Type: application/json' | jq -r '.result')
    echo "$(printf "%d" $HEX_NUM)"
  }
  while [ $TRANSFER_RUNS -lt $MAX_RUNS ]; do
    if [ $(balance) -eq $EXPECTED_BALANCE ]; then
      echo "Transfer success"
      break
    else
      sleep 1
    fi
    TRANSFER_RUNS=$((TRANSFER_RUNS+1))
  done
  if [ $TRANSFER_RUNS -eq $MAX_RUNS ]; then
    echo "Transfer failure"
    exit 1
  fi

  echo "Testing soft commits..."
  SOFT_RUNS=0
  soft() {
    HEX_NUM=$(curl -X POST $ETH_RPC_URL -s -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["safe", false],"id":1}' -H 'Content-Type: application/json' | jq -r '.result.number')
    echo "$(printf "%d" $HEX_NUM)"
  }
  while [ $SOFT_RUNS -lt $MAX_RUNS ]; do
    if [ $(soft) -gt 0 ]; then
      echo "Soft commit success"
      break
    else
      sleep 1
    fi
    SOFT_RUNS=$((SOFT_RUNS+1))
  done

  echo "Testing finalization..."
  FINALIZED_RUNS=0
  finalized() {
    HEX_NUM=$(curl -X POST $ETH_RPC_URL -s -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["finalized", false],"id":1}' -H 'Content-Type: application/json' | jq -r '.result.number')
    echo "$(printf "%d" $HEX_NUM)"
  }
  while [ $FINALIZED_RUNS -lt $MAX_RUNS ]; do
    if [ $(finalized) -gt 0 ]; then
      echo "Finalized success"
      exit 0
    else
      sleep 1
    fi
    FINALIZED_RUNS=$((FINALIZED_RUNS+1))
  done
  echo "Finalization failure"
  exit 1

delete-smoke-test:
  just delete celestia-local
  just delete sequencer
  just delete rollup
