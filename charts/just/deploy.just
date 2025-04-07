set allow-duplicate-recipes

import 'defaults.just'


# Set the working directory for all recipes to the project's root.
set working-directory := "../../"


_default_deploy:
  @just --list deploy


########################
## Deployment Recipes ##
########################


# Deploy All
#############
[doc("
Deploys cluster, ingress controller, and entire Astria stack, including rollup with
Astrotrek and sequencer faucet.
")]
all:
  @just deploy cluster
  @just deploy ingress-controller
  @just wait-for ingress-controller
  @just deploy astria-local
  @just deploy astrotrek
  @just deploy::deploy-chart "sequencer-faucet"
  @just deploy rollup


# Deploy Astria Local
######################
[doc("
Deploys local Sequencer and Celestia instance.
Usage:
  just deploy astria-local <NAMESPACE> (default: 'astria-dev-cluster')
")]
astria-local namespace=defaultNamespace: (celestia-local namespace) (sequencer)


# Deploy local Celestia instance
#################################
[doc("
Deploys local Celestia instance.
Usage:
  just deploy celestia-local <NAMESPACE> (default: 'astria-dev-cluster')
")]
celestia-local namespace=defaultNamespace: (deploy-chart "celestia-local" namespace)


# Deploy Cluster
#################
hubbleEnabled := "false"
[doc("
Deploys a local Kubernetes cluster.
Usage:
  just deploy cluster <NAMESPACE> <ENABLE_HUBBLE> (defaults: 'astria-dev-cluster', 'false')
")]
cluster namespace=defaultNamespace enableHubble=hubbleEnabled:
  @kind create cluster --image kindest/node:v1.28.9 --config dev/kubernetes/kind-cluster-config.yml
  @just deploy::helm-add-if-not-exist cilium https://helm.cilium.io/ > /dev/null
  @echo "Installing cilium..."
  @helm install cilium cilium/cilium --version 1.15.5 \
      -f dev/values/cilium.yml \
      {{ if enableHubble == "true" { '--set hubble.enabled=true --set hubble.relay.enabled=true --set hubble.ui.enabled=true --set hubble.ui.ingress.enabled=true' } else { '' } }} \
      --namespace kube-system \
      --labels core=true
  @kubectl create namespace {{namespace}}


# Deploy Local Metrics
#######################
[doc("
Deploys the local metrics serveras defined in '/dev/kubernetes/metrics-server-local.yml'.
")]
local-metrics:
  @kubectl apply -f dev/kubernetes/metrics-server-local.yml


# Deploy Ingress Controller
############################
[doc("
Deploys an ingress controller.
")]
ingress-controller:
  @echo "Deploying ingress controller..."
  @kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml > /dev/null


# Deploy Graph Node
####################
[doc("
Deploys a local graph node.
Usage:
  just deploy graph-node <NAMESPACE> (default: 'astria-dev-cluster')
")]
graph-node namespace=defaultNamespace: (deploy-chart "graph-node" namespace)


# Deploy Secrets Store
#######################
[doc("
Deploys a local secrets store if it doesn't already exist.
")]
secrets-store:
  @echo "Deploying secrets store..."
  @just deploy::helm-add-if-not-exist secrets-store-csi-driver https://kubernetes-sigs.github.io/secrets-store-csi-driver/charts
  @helm install csi-secrets-store-chart secrets-store-csi-driver/secrets-store-csi-driver --namespace kube-system


# Deploy Sequencer
###################
[doc("
Deploys a local Sequencer node from 'dev/values/validators'.
Usage:
  just deploy sequencer <NAME> (default: 'single')
")]
sequencer name=validatorName:
  @echo "Deploying sequencer '{{name}}'..."
  @helm dependency update charts/sequencer > /dev/null
  @helm install \
    -f dev/values/validators/all.yml \
    -f dev/values/validators/{{name}}.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    -n astria-validator-{{name}} --create-namespace \
    {{name}}-sequencer-chart charts/sequencer


# Deploy Sequencers
####################
[doc("
Deploys a local Sequencer network with 3 nodes defined at 'dev/values/validators/node[0..2].yml'.
")]
sequencers:
  @echo "Deploying sequencers..."
  @just deploy sequencer "node0"
  @just deploy sequencer "node1"
  @just deploy sequencer "node2"
  @just deploy sequencer "node3"
  @just deploy sequencer "node4"


# Deploy Astrotrek
###################
[doc("
Deploys a local Astrotrek instance.
")]
astrotrek:
  @echo "Deploying Astrotrek..."
  @helm dependency update charts/astrotrek > /dev/null
  @helm install astrotrek-chart charts/astrotrek \
    -n astria-dev-cluster \


# Deploy Hermes
################
[doc("
Deploys a local Hermes instance.
")]
hermes-local:
  @echo "Deploying Hermes..."
  @helm install hermes-local-chart charts/hermes \
    -n astria-dev-cluster \
    -f dev/values/hermes/local.yaml


# Deploy Rollup
################
[doc("
Deploys a geth rollup chain + faucet + blockscout + ingress with defaults running
against local network, along with a bridge withdawer. Local Sequencer and Celestia
networks must be running before deploying the rollup (both can be deployed with
`just deploy astria-local`).
Note: default values file can be found in `helm/rollup/values.yaml`
Usage:
  just deploy rollup <ROLLUP_NAME> <NETWORK_ID> (defaults: 'astria', '')
")]
rollup rollupName=defaultRollupName networkId=defaultNetworkId:
  @echo "Deploying rollup '{{rollupName}}'..."
  @helm dependency update charts/evm-stack > /dev/null
  @helm install \
    {{ if rollupName != '' { replace('--set config.rollup.name=# --set celestia-node.config.labelPrefix=#', '#', rollupName) } else { '' } }} \
    {{ if networkId != '' { replace('--set config.rollup.networkId=#', '#', networkId) } else { '' } }} \
    -f dev/values/rollup/dev.yaml \
    {{rollupName}}-chain-chart charts/evm-stack --namespace astria-dev-cluster


# Deploy Flame Dev Rollup
##########################
[doc("
Deploys local Flame instance. Local Sequencer and Celestia networks must be running
before deploying the rollup.
Usage:
  just deploy flame-dev-rollup <ROLLUP_NAME> <NETWORK_ID> (defaults: 'astria', '')
")]
flame-dev-rollup rollupName=defaultRollupName networkId=defaultNetworkId:
  @echo "Deploying Flame rollup '{{rollupName}}'..."
  @helm dependency update charts/evm-stack > /dev/null
  @helm install \
    {{ if rollupName != '' { replace('--set config.rollup.name=# --set celestia-node.config.labelPrefix=#', '#', rollupName) } else { '' } }} \
    {{ if networkId != '' { replace('--set config.rollup.networkId=#', '#', networkId) } else { '' } }} \
    -f dev/values/rollup/flame-dev.yaml \
    {{rollupName}}-chain-chart charts/evm-stack --namespace astria-dev-cluster


# FIXME: This deployment currently fails
# Deploy Auctioneer
####################
[doc("
Deploys local auctioneer.
")]
auctioneer:
  @echo "Deploying auctioneer..."
  @helm dependency update charts/auctioneer > /dev/null
  @helm install auctioneer-chart charts/auctioneer \
      --namespace astria-dev-cluster \
      -f dev/values/auctioneer/values.yaml


# Deploy EVM Bridge Withdrawer
###############################
[doc("
Deploys local EVM Bridge Withdrawer.
")]
bridge-withdrawer:
  @echo "Deploying bridge-withdrawer..."
  @helm install evm-bridge-withdrawer-chart charts/evm-bridge-withdrawer \
    --namespace astria-dev-cluster \
    -f dev/values/withdrawer/values.yaml


# FIXME: This deployment currently fails
# Deploy Hyperlane Agents
##########################
[doc("
Deploys local Hyperlane agents.
Usage:
  just deploy hyperlane-agents <ROLLUP_NAME> <AGENT_CONFIG> <RELAYER_PRIVATE_KEY> <VALIDATOR_PRIVATE_KEY>
  (defaults: 'astria', '', '', '')
")]
hyperlane-agents rollupName=defaultRollupName agentConfig=defaultHypAgentConfig relayerPrivateKey=defaultHypRelayerPrivateKey validatorPrivateKey=defaultHypValidatorPrivateKey:
  helm install --debug \
    {{ if rollupName          != '' { replace('--set config.name=# --set global.namespace=#-dev-cluster', '#', rollupName) } else { '' } }} \
    {{ if agentConfig         != '' { replace('--set config.agentConfig=#', '#', agentConfig) } else { '' } }} \
    {{ if relayerPrivateKey   != '' { replace('--set config.relayer.privateKey=#', '#', relayerPrivateKey) } else { '' } }} \
    {{ if validatorPrivateKey != '' { replace('--set config.validator.privateKey=#', '#', validatorPrivateKey) } else { '' } }} \
    {{rollupName}}-hyperlane-agents-chart charts/hyperlane-agents --namespace astria-dev-cluster


# Deploy Smoke Test
####################
[doc("
Deploys all components necessary for local end-to-end (including rollup and Celestia)
smoke test and awaits their readiness.
Usage:
  just deploy smoke-test <TAG> (default: '')
")]
smoke-test tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @helm dependency update charts/sequencer > /dev/null
  @helm dependency update charts/evm-stack > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f dev/values/validators/all.yml \
    -f dev/values/validators/single.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    {{ if tag != '' { replace('--set images.sequencer.tag=# --set sequencer-relayer.images.sequencerRelayer.tag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart charts/evm-stack -f dev/values/rollup/dev.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=# --set evm-bridge-withdrawer.images.evmBridgeWithdrawer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set postgresql.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  @just wait-for celestia-local
  @just wait-for sequencer
  @just wait-for rollup


# Deploy Smoke CLI
###################
[doc("
Deploys all components necessary for local end-to-end smoke test without EVM Bridge
Withdrawer and awaits their readiness.
Usage:
  just deploy smoke-cli <TAG> (default: '')
")]
smoke-cli tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @helm dependency update charts/sequencer > /dev/null
  @helm dependency update charts/evm-stack > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f dev/values/validators/all.yml \
    -f dev/values/validators/single.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    {{ if tag != '' { replace('--set images.sequencer.tag=# --set sequencer-relayer.images.sequencerRelayer.tag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart charts/evm-stack -f dev/values/rollup/dev.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set postgresql.enabled=false \
    --set evm-bridge-withdrawer.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  @just wait-for celestia-local
  @just wait-for sequencer
  @just wait-for rollup {{defaultRollupName}} "false"
  @sleep 10


# Deploy IBC Test
##################
[doc("
Deploys all components necessary for smoke test via IBC transactions and awaits
their readiness.
Usage:
  just deploy ibc-test <TAG> (default: '')
")]
ibc-test tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @helm dependency update charts/sequencer > /dev/null
  @helm dependency update charts/evm-stack > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f dev/values/validators/all.yml \
    -f dev/values/validators/single.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    {{ if tag != '' { replace('--set images.sequencer.tag=# --set sequencer-relayer.images.sequencerRelayer.tag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart charts/evm-stack \
    -f dev/values/rollup/dev.yaml \
    -f dev/values/rollup/ibc-bridge-test.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=# --set evm-bridge-withdrawer.images.evmBridgeWithdrawer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set postgresql.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  @echo "Deploying Hermes..." && just deploy hermes-local > /dev/null
  @just wait-for celestia-local
  @just wait-for sequencer
  @just wait-for rollup
  @just wait-for hermes-local


# Deploy IBC Test Without Native Asset
#######################################
[doc("
Deploys all components necessary for IBC test with no native asset defined and awaits
their readiness.
Usage:
  just deploy ibc-test-without-native <TAG> (default: '')
")]
ibc-test-without-native tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..."
  @helm install celestia-local-chart charts/celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
  @helm dependency update charts/sequencer > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f dev/values/validators/all-without-native.yml \
    -f dev/values/validators/single.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    {{ if tag != '' { replace('--set images.sequencer.tag=# --set sequencer-relayer.images.sequencerRelayer.tag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @echo "Deploying Hermes..."
  @just deploy hermes-local > /dev/null
  @just wait-for celestia-local
  @just wait-for sequencer
  @just wait-for hermes-local


# Deploy IBC Test Timeout Refund
#################################
[doc("
Deploys all components necessary for IBC test with a timeout on receipt of a refund
and awaits their readiness.
Usage:
  just deploy ibc-test-timeout-refund <TAG> (default: '')
")]
ibc-test-timeout-refund tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." > /dev/null
  @helm install celestia-local-chart charts/celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
  @helm dependency update charts/sequencer > /dev/null
  @helm dependency update charts/evm-stack > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f dev/values/validators/all.yml \
    -f dev/values/validators/single.yml \
    -f dev/values/validators/aspen.upgrade.yml \
    {{ if tag != '' { replace('--set images.sequencer.tag=# --set sequencer-relayer.images.sequencerRelayer.tag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart charts/evm-stack \
    -f dev/values/rollup/dev.yaml \
    -f dev/values/rollup/ibc-bridge-test.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set evm-bridge-withdrawer.enabled=false \
    --set postgresql.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  @echo "Deploying Hermes..."
  @just deploy hermes-local > /dev/null
  @just wait-for celestia-local
  @just wait-for sequencer
  @just wait-for rollup {{defaultRollupName}} "false"
  @just wait-for hermes-local


# Generate System-Test Protos
##############################
[doc("
Generates Python files from the protobuf sources for the system-tests.
")]
generate-system-test-protos:
  buf generate --template system-tests/buf.gen.yaml


# Deploy Upgrade Test
######################
[doc("
Deploys all components necessary for running the sequencer upgrade test in the
`system-tests` folder.
")]
upgrade-test: generate-system-test-protos
  @echo "Deploying ingress controller..." && just deploy ingress-controller > /dev/null
  @echo "Waiting for ingress controller to be ready..." && just wait-for ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @just wait-for celestia-local


#####################
## Private Recipes ##
#####################

[private]
deploy-chart chart namespace=defaultNamespace:
  @helm install {{chart}}-chart charts/{{chart}} --namespace {{namespace}} --create-namespace

[private]
helm-add-if-not-exist repo url:
  @helm repo list | grep -q {{repo}} || helm repo add {{repo}} {{url}}
