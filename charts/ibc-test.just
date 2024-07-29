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
run celestiaHome pathToCelestiaAppd=default_celestia_appd :
  #!/usr/bin/env bash

  initial_balance=$(just evm-get-balance {{evm_destination_address}})

  # Create a bridge account on the sequencer
  just init-ibc-bridge {{ sequencer_tia_bridge_pkey }} transfer/channel-0/utia nria

  # Load the private key of the Celestia dev account to issue transfers
  just ibc-test _load-celestia-key "{{ celestiaHome}}" "{{ pathToCelestiaAppd }}"

  # Execute the transfer from Celestia to the Rollup
  just ibc-test _do-ibc-transfer "{{ celestiaHome}}" "{{ pathToCelestiaAppd }}"

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

  just evm-send-raw-transaction {{ evm_withdraw_tx_raw }}
  expected_evm_balance="$current_balance - 1000000000"
  for i in {1..50}
  do
    tx_receipt=$(just evm-get-transaction-receipt {{ evm_withdraw_tx_hash }})
    current_balance=$(just evm-get-balance {{evm_destination_address}})
    if [[ $tx_receipt != "null" ]]; then
      echo "Transaction receipt found: $tx_receipt"
      if (( expected_evm_balance == $current_balance )); then
        echo "EVM balance updated correctly"
        break
      else
        echo "EVM balance not updated correctly"
        exit 1
      fi
      break
    else
      sleep 1
    fi
  done
    

bridge_address := "astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr"
celestia_dev_account_address := "celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96"
celestia_dev_account_key_name := "dev"
celestia_dev_account_mnemonic := "enrich avocado local net will avoid dizzy truth column excuse ready lesson"
celestia_chain_id := "celestia-local-0"
celestia_node_url := "http://rpc.app.celestia.localdev.me:80"
sequencer_tia_bridge_pkey := "6015fbe1c365d3c5ef92dc891db8c5bb26ad454bec2db4762b96e9f8b2430285"
keyring_backend := "test"
celestia_desitnation_address := "0x4a58639fb5458e65e4fa917ff951c390292c24a1"

# This is the same address as used in deploy.just
evm_destination_address := "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"

# all in units of utia
transfer_amount := "53000"
transfer_fees := "26000"

default_celestia_appd := "celestia-appd"
[no-cd]
_load-celestia-key celestiaHome pathToCelestiaAppd=default_celestia_appd:
  #!/usr/bin/env bash
  pwd
  "{{pathToCelestiaAppd}}" keys add {{ celestia_dev_account_key_name }} \
    --home "{{celestiaHome}}" \
    --keyring-backend="{{ keyring_backend }}" \
    --recover <<< "{{ celestia_dev_account_mnemonic }}"

[no-cd]
_do-ibc-transfer celestiaHome pathToCelestiaAppd=default_celestia_appd:
  echo "Performing IBC transfer..."
  "{{pathToCelestiaAppd}}" tx ibc-transfer transfer \
    transfer \
    channel-0 \
    {{ bridge_address }} \
    "{{ transfer_amount }}utia" \
    --memo="{\"rollupDepositAddress\":\"{{ evm_destination_address }}\"}" \
    --chain-id="{{ celestia_chain_id }}" \
    --node="{{ celestia_node_url }}" \
    --from="{{ celestia_dev_account_address }}" \
    --fees="{{ transfer_fees }}utia" \
    --yes \
    --log_level=debug \
    --home "{{celestiaHome}}" \
    --keyring-backend="{{ keyring_backend }}"

evm_withdraw_tx_hash := "0x3f52d86dab0f0a574e23c033dfa2169dae667e4c0e80f0f430d8a29ffed81b8c"
evm_withdraw_tx_raw := "0xf9012f80843b9aca07828fe294a58639fb5458e65e4fa917ff951c390292c24a1585e8d4a51000b8c4a996e020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000002f63656c657374696131786b3676686d72706a71306d6a746a64667333656d66347979796b38366d7535796e656b666a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000820a95a0dc05b5021f66c2a77605e39671687960bc3c0871302426453e0f2736ded33680a059b90c77d82f759e200a6c4e9df81dea9f8b9ab288a84f2a70938e4c2dde8b5d"