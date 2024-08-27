defaultTag := ""

delete:
  -just delete celestia-local
  -just delete sequencer
  -just delete hermes-local
  -just delete rollup

@deploy tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..." && just deploy celestia-local > /dev/null
  helm dependency update ./sequencer > /dev/null
  helm dependency update ./evm-stack > /dev/null
  echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart ./sequencer \
    -f ../dev/values/validators/all.yml \
    -f ../dev/values/validators/single.yml \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  just wait-for-sequencer > /dev/null
  echo "Starting EVM rollup..." && helm install -n astria-dev-cluster astria-chain-chart ./evm-stack \
    -f ../dev/values/rollup/dev.yaml \
    -f ../dev/values/rollup/ibc-bridge-test.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=# --set evm-bridge-withdrawer.images.evmBridgeWithdrawer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set postgresql.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  just wait-for-rollup > /dev/null
  echo "Deploying Hermes"
  just deploy hermes-local > /dev/null
  kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=300s

[no-cd]
run:
  #!/usr/bin/env bash
  set -e

  initial_balance=$(just evm-get-balance {{evm_destination_address}})

  # Create a bridge account on the sequencer
  just init-ibc-bridge {{ sequencer_tia_bridge_pkey }} transfer/channel-0/utia nria

  # Execute the transfer from Celestia to the Rollup
  just ibc-test _do-ibc-transfer

  # Multiplication factor is 10^-6 (utia to tia) * 10^18 (rollup factor) = 10^12
  let expected_balance="$initial_balance + {{ transfer_amount }} * 10**12"

  for i in {1..50}
  do
    current_balance=$(just evm-get-balance {{evm_destination_address}})
    echo "check $i, balance: $current_balance, expected: $expected_balance"
    if (( expected_balance == $current_balance )); then
      expected_balance_found="1"
      break
    else
      sleep 1
    fi
  done
  if [[ -z $expected_balance_found ]]; then
    echo "expected balance was not found; IBC transfer from Celestia to the Rollup failed"
    exit 1
  fi
    

bridge_address := "astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr"
celestia_dev_account_address := "celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96"
celestia_chain_id := "celestia-local-0"
celestia_node_url := "http://rpc.app.celestia.localdev.me:80"
sequencer_tia_bridge_pkey := "6015fbe1c365d3c5ef92dc891db8c5bb26ad454bec2db4762b96e9f8b2430285"
keyring_backend := "test"

# This is the same address as used in deploy.just
evm_destination_address := "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"

# all in units of utia
transfer_amount := "53000"
transfer_fees := "26000"

# TODO: move this to deploy.just so that defaultNamespace need not be redefined
defaultNamespace := "astria-dev-cluster"
[no-cd]
_do-ibc-transfer namespace=defaultNamespace:
  echo "Performing IBC transfer..."
  kubectl exec -n {{ namespace }} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd tx ibc-transfer transfer \
    transfer \
    channel-0 \
    {{ bridge_address }} \
    "{{ transfer_amount }}utia" \
    --memo="{\"rollupDepositAddress\":\"{{ evm_destination_address }}\"}" \
    --chain-id="{{ celestia_chain_id }}" \
    --from="{{ celestia_dev_account_address }}" \
    --fees="{{ transfer_fees }}utia" \
    --yes \
    --log_level=debug \
    --home /home/celestia \
    --keyring-backend="{{ keyring_backend }}"'
