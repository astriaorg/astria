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
  @echo "Deploying Hermes and creating IBC channel..."
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

# deploy a bridge tester chart that runs a job to test the ibc bridge
deploy-bridge-tester namespace=defaultNamespace:
  helm install --debug bridge-tester-chart ./charts/bridge-test --namespace {{namespace}}

# delete the bridge tester release
delete-bridge-tester namespace=defaultNamespace:
  helm uninstall bridge-tester-chart --namespace {{namespace}}

celestia_dev_account_mnemonic := 'enrich avocado local net will avoid dizzy truth column excuse ready lesson'

add-key:
  echo "{{celestia_dev_account_mnemonic}}" | \
  ~/code/astria/projects/get-hermes-running/celestia-app/celestia-appd keys add \
  "dev-account" \
  --home "./tmp-celestia-home/celestia" \
  --keyring-backend="test" \
  --recover

ibc-tx:
  ~/code/astria/projects/get-hermes-running/celestia-app/celestia-appd tx ibc-transfer transfer \
    transfer \
    channel-0 \
    astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr \
    53000utia \
    --memo=0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30 \
    --chain-id=celestia-local-0 \
    --node=http://rpc.app.celestia.localdev.me:80 \
    --from=celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96 \
    --fees=420utia \
    --log_level=debug \
    --home="./tmp-celestia-home/celestia" \
    --keyring-backend="test"
