# deploy infra for an ibc bridge smoke test
deploy-ibc-test-infra tag=defaultTag:
  @echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  @just wait-for-ingress-controller > /dev/null
  @echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  @helm dependency update ./charts/sequencer > /dev/null
  @helm dependency update ./charts/evm-rollup > /dev/null
  @echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart charts/sequencer \
    -f ./dev/values/validators/single.yml \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  @just wait-for-sequencer > /dev/null
  @echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart ./charts/evm-rollup \
    -f ./dev/values/rollup/dev.yaml \
    -f ./dev/values/rollup/ibc-bridge-test.yaml \
    {{ if tag != '' { replace('--set images.conductor.devTag=# --set images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set config.blockscout.enabled=false \
    --set config.faucet.enabled=false > /dev/null
  @just wait-for-rollup > /dev/null
  @echo "Deploying hermes..."
  @just deploy hermes-local > /dev/null
  # TODO - need to wait on hermes. do i need to implement a readinessProbe for hermes chart?
  #@just deploy bridge-tester

# delete infra used for the ibc bridge test
delete-ibc-test-infra:
  just delete celestia-local
  just delete sequencer
  just delete rollup
  just delete hermes-local
  just delete bridge-tester
