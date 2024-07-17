# deploy infra for an ibc bridge smoke test
@deploy-ibc-test-infra tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  helm dependency update ./charts/sequencer > /dev/null
  helm dependency update ./charts/evm-rollup > /dev/null
  echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f ./dev/values/validators/all.yml \
    -f ./dev/values/validators/single.yml \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  just wait-for-sequencer > /dev/null
  echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart ./charts/evm-rollup \
    -f ./dev/values/rollup/dev.yaml \
    -f ./dev/values/rollup/ibc-bridge-test.yaml \
    {{ if tag != '' { replace('--set images.conductor.devTag=# --set images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set config.blockscout.enabled=false \
    --set config.faucet.enabled=false > /dev/null
  just wait-for-dev-rollup > /dev/null
  echo "Deploying Hermes and creating IBC channel..."
  just deploy hermes-local > /dev/null
  kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=300s

# delete infra used for the ibc bridge test
delete-ibc-test-infra:
  -just delete celestia-local
  -just delete sequencer
  -just delete rollup
  -just delete hermes-local
  -just delete bridge-tester

# deploy a bridge tester chart that runs a job to test the ibc bridge
deploy-bridge-tester tag='local' namespace=defaultNamespace:
  helm install --debug bridge-tester-chart ./charts/bridge-test \
    --namespace {{namespace}} \
    --set bridgeTesterUtilityImage=ghcr.io/astriaorg/bridge-tester-utility:{{tag}}

# delete the bridge tester release
delete-bridge-tester namespace=defaultNamespace:
  helm uninstall bridge-tester-chart --namespace {{namespace}}

###########
# helpers #
###########

# NOTE - can't build for darwin because base images don't support it
default_target_platform := 'linux/amd64'

# build bridge tester utility image, load into cluster
build-and-load-bridge-tester-image tag='local' target_platform=default_target_platform:
  docker buildx build \
    --load \
    --platform {{target_platform}} \
    -f dev/containerfiles/bridgetesterutility.Dockerfile \
    -t ghcr.io/astriaorg/bridge-tester-utility:{{tag}} .
  just load-image ghcr.io/astriaorg/bridge-tester-utility:{{tag}}

# build the astria-geth image and load into cluster. NOTE - assumes astria-geth is sibling directory to monorepo
build-and-load-astria-geth:
  @echo "building astria-geth local docker image..."
  cd ../astria-geth && docker buildx build -f Dockerfile -t ghcr.io/astriaorg/astria-geth:local .
  just load-image ghcr.io/astriaorg/astria-geth:local

# load astria-go and astria-geth images into cluster
load-images:
  #just load-image ghcr.io/astriaorg/astria-go:local
  just load-image ghcr.io/astriaorg/bridge-tester-utility:local
  just load-image ghcr.io/astriaorg/astria-geth:local
