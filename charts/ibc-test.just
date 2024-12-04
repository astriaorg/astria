_default:
  @just --list ibc-test

defaultTag := ""

delete:
  -just delete celestia-local
  -just delete sequencer
  -just delete hermes-local
  -just delete rollup

@deploy tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..."
  helm install celestia-local-chart ./celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
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
  kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=480s

@deploy-without-native tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..."
  helm install celestia-local-chart ./celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
  helm dependency update ./sequencer > /dev/null
  echo "Setting up single astria sequencer..." && helm install \
    -n astria-validator-single single-sequencer-chart ./sequencer \
    -f ../dev/values/validators/all-without-native.yml \
    -f ../dev/values/validators/single.yml \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null
  just wait-for-sequencer > /dev/null
  echo "Deploying Hermes"
  just deploy hermes-local > /dev/null
  kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=480s

@deploy-timeout tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..." > /dev/null
  helm install celestia-local-chart ./celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
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
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set composer.images.composer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set evm-bridge-withdrawer.enabled=false \
    --set postgresql.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  just wait-for-dev-rollup > /dev/null
  echo "Deploying Hermes"
  just deploy hermes-local > /dev/null
  kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=480s

[no-cd]
run-without-native tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  ASTRIA_CLI_IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"

  # Execute the transfer from Celestia to the Rollup
  just ibc-test _do-ibc-transfer {{defaultNamespace}} {{sequencer_sudo_address}}

  # check that sequencer balance updated correctly
  EXPECTED_BALANCE=$(echo "1 * {{transfer_amount}}" | bc)
  for i in {1..50}
  do
    BALANCE=$(docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer account balance {{sequencer_sudo_address}} --sequencer-url {{sequencer_rpc_url}}  | awk '/transfer\/channel-0\/utia/{print $(NF-1)}')
    echo "check $i, balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      expected_sequencer_balance_found="1"
      break
    else
      sleep 1
    fi
  done
  if [[ -z $expected_sequencer_balance_found ]]; then
    echo "expected sequencer balance was not found after IBC transfer; IBC transfer with compat address failed"
    exit 1
  fi

  # Add transfer/channel-0/utia as allowed fee-asset
  docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer sudo fee-asset add --private-key {{sequencer_sudo_pkey}} --asset transfer/channel-0/utia --sequencer-url {{sequencer_rpc_url}} --sequencer.chain-id {{sequencer_chain_id}}

  # TODO: query allowd fee asset verifying succefull addition

[no-cd]
run tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  initial_balance=$(just evm-get-balance {{evm_destination_address}})
  initial_celestia_balance=$(just ibc-test get-celestia-balance)

  # Create a bridge account on the sequencer
  just init-ibc-bridge {{sequencer_tia_bridge_priv_key}} transfer/channel-0/utia nria {{tag}}

  # Execute the transfer from Celestia to the Rollup
  just ibc-test _do-rollup-ibc-transfer

  # Multiplication factor is 10^-6 (utia to tia) * 10^18 (rollup factor) = 10^12
  let expected_balance="$initial_balance + {{transfer_amount}} * 10**12"
  # `bc` because very large number
  expected_celestia_balance=$(echo "$initial_celestia_balance - {{transfer_amount}} - {{transfer_fees}}" | bc)

  # check that evm balance updated correctly
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

  # Execute the transfer from Celstia to sequencer with compat address
  just ibc-test _do-ibc-transfer {{defaultNamespace}} {{compat_address}}

  # check that sequencer balance updated correctly
  ASTRIA_CLI_IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"
  EXPECTED_BALANCE=$(echo "1 * {{transfer_amount}}" | bc)
  for i in {1..50}
  do
    BALANCE=$(docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer account balance {{sequencer_address}} --sequencer-url {{sequencer_rpc_url}}  | awk '/transfer\/channel-0\/utia/{print $(NF-1)}')
    echo "check $i, balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      expected_sequencer_balance_found="1"
      break
    else
      sleep 1
    fi
  done
  if [[ -z $expected_sequencer_balance_found ]]; then
    echo "expected sequencer balance was not found after IBC transfer; IBC transfer with compat address failed"
    exit 1
  fi

  # test IBC withdrawal from the EVM to Celestia
  post_deposit_evm_balance=$(just evm-get-balance {{evm_destination_address}})
  let expected_evm_balance="$post_deposit_evm_balance - 1000000000000"

  initial_celestia_balance=$(just ibc-test get-celestia-balance)
  # `bc` because very large number
  # WITHDRAW_AMOUNT / 10^(18-6) = 1
  expected_celestia_balance=$(echo "$initial_celestia_balance + 1" | bc)
  echo "Initial Celestia balance $initial_celestia_balance"

  echo "Performing IBC withdrawal from the EVM to Celestia..."
  just evm-send-raw-transaction {{evm_withdraw_tx_raw}}

  # check that evm balance updated correctly
  for i in {1..50}
  do
    tx_receipt=$(just evm-get-transaction-receipt {{evm_withdraw_tx_hash}})
    if [[ $tx_receipt != "null" ]]; then
      echo "Transaction receipt found: $tx_receipt"
      tx_receipt_found="1"
      current_balance=$(just evm-get-balance {{evm_destination_address}})
      if (( $expected_evm_balance == $current_balance )); then
        echo "EVM balance updated correctly - expected: $expected_evm_balance, actual: $current_balance"
        break
      else
        echo "EVM balance not updated correctly - expected: $expected_evm_balance, actual: $current_balance"
        exit 1
      fi
      break
    else
      sleep 1
    fi
  done
  if [[ -z $tx_receipt_found ]]; then
    echo "Transaction receipt not found; IBC transfer from the Rollup to Celestia failed"
    exit 1
  fi

  # check that celestia balance updated correctly
  for i in {1..50}
  do
    current_celestia_balance=$(just ibc-test get-celestia-balance)
    echo "check $i, balance: $current_celestia_balance, expected: $expected_celestia_balance"
    if (( $expected_celestia_balance == $current_celestia_balance )); then
      expected_celestia_balance_after_wd="1"
      break
    else
      sleep 1
    fi
  done
  if [[ -z $expected_celestia_balance_after_wd ]]; then
    echo "expected celestia balance was not found after withdraw; IBC transfer from the Rollup to Celestia failed"
    exit 1
  fi

sequencer_tia_bridge_address := "astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr"
eth_ws_url := "ws://ws-executor.astria.localdev.me/"
evm_contract_address := "0xA58639fB5458e65E4fA917FF951C390292C24A15"
[no-cd]
run-timeout tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  initial_balance=$(just evm-get-balance {{evm_destination_address}})
  initial_celestia_balance=$(just ibc-test get-celestia-balance)

  # Create a bridge account on the sequencer
  just init-ibc-bridge {{sequencer_tia_bridge_priv_key}} transfer/channel-0/utia nria {{tag}}

  # Execute the transfer from Celestia to the Rollup
  just ibc-test _do-rollup-ibc-transfer

  # Multiplication factor is 10^-6 (utia to tia) * 10^18 (rollup factor) = 10^12
  let expected_balance="$initial_balance + {{transfer_amount}} * 10**12"

  # `bc` because very large number
  expected_celestia_balance=$(echo "$initial_celestia_balance - {{transfer_amount}} - {{transfer_fees}}" | bc)

  # check that evm balance updated correctly
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

  # Execute the transfer from Celstia to sequencer with compat address
  just ibc-test _do-ibc-transfer {{defaultNamespace}} {{compat_address}}

  # check that sequencer balance updated correctly
  ASTRIA_CLI_IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"
  EXPECTED_BALANCE=$(echo "1 * {{transfer_amount}}" | bc)
  for i in {1..50}
  do
    BALANCE=$(docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer account balance {{sequencer_address}} --sequencer-url {{sequencer_rpc_url}}  | awk '/transfer\/channel-0\/utia/{print $(NF-1)}')
    echo "check $i, balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      expected_sequencer_balance_found="1"
      break
    else
      sleep 1
    fi
  done
  if [[ -z $expected_sequencer_balance_found ]]; then
    echo "expected sequencer balance was not found after IBC transfer; IBC transfer with compat address failed"
    exit 1
  fi

  # test IBC withdrawal from the EVM to Celestia
  post_deposit_evm_balance=$(just evm-get-balance {{evm_destination_address}})
  let expected_evm_balance="$post_deposit_evm_balance - 1000000000000"

  initial_celestia_balance=$(just ibc-test get-celestia-balance)
  # `bc` because very large number
  # WITHDRAW_AMOUNT / 10^(18-6) = 1
  expected_celestia_balance=$(echo "$initial_celestia_balance + 1" | bc)
  echo "Initial Celestia balance $initial_celestia_balance"

  echo "Performing IBC withdrawal from the EVM to Celestia..."
  just evm-send-raw-transaction {{evm_withdraw_tx_raw}}

  # check that evm balance updated correctly
  for i in {1..50}
  do
    tx_receipt=$(just evm-get-transaction-receipt {{evm_withdraw_tx_hash}})
    if [[ $tx_receipt != "null" ]]; then
      echo "Transaction receipt found: $tx_receipt"
      tx_receipt_found="1"
      current_balance=$(just evm-get-balance {{evm_destination_address}})
      if (( $expected_evm_balance == $current_balance )); then
        echo "EVM balance updated correctly - expected: $expected_evm_balance, actual: $current_balance"
        break
      else
        echo "EVM balance not updated correctly - expected: $expected_evm_balance, actual: $current_balance"
        exit 1
      fi
      break
    else
      sleep 1
    fi
  done
  if [[ -z $tx_receipt_found ]]; then
    echo "Transaction receipt not found; IBC transfer from the Rollup to Celestia failed"
    exit 1
  fi

  CURRENT_BLOCK_HEX=$(just evm-get-block-by-number latest | jq -r '.number')
  CURRENT_BLOCK=$(just hex-to-dec $CURRENT_BLOCK_HEX)

  # Using a docker volume to handle both passing in a private key & the output file
  docker volume create cli-test-withdrawals
  docker run --rm -v cli-test-withdrawals:/data alpine sh -c "echo '{{sequencer_tia_bridge_priv_key}}' > /data/key"
  docker run --rm -v cli-test-withdrawals:/astria --network host $ASTRIA_CLI_IMAGE bridge collect-withdrawals \
    --rollup-endpoint {{eth_ws_url}} \
    --contract-address {{evm_contract_address}} \
    --from-rollup-height 1 \
    --to-rollup-height $CURRENT_BLOCK \
    --ics20-asset-to-withdraw "transfer/channel-0/utia" \
    --bridge-address {{sequencer_tia_bridge_address}} \
    --output "/astria/tempfile" \
    --force

  # There are some acknowledgements flying back and forth which make creating a height
  # that is a timeout without being stale with a client update a challenge without
  # waiting for acknowledgements to settle. This is a hack.
  echo "Waiting for 5 seconds to allow for acknowledgements to settle..."
  sleep 5

  # Grabbing the current celestia block height and resetting the timeout height of the withdrawal
  # to one lower to force an immediate timeout which should create a refund event.
  CELESTIA_BLOCK_HEIGHT=$(just ibc-test get-celestia-height)
  docker run --rm -v cli-test-withdrawals:/data alpine sh -c "sed -i 's/\"timeoutHeight\":{\"revisionNumber\":\"18446744073709551615\",\"revisionHeight\":\"18446744073709551615\"}/\"timeoutHeight\":{\"revisionNumber\":\"0\",\"revisionHeight\":\"$(($CELESTIA_BLOCK_HEIGHT - 1))\"}/g' /data/tempfile"
  # Send the withdrawals
  docker run --rm -v cli-test-withdrawals:/astria --network host $ASTRIA_CLI_IMAGE bridge submit-withdrawals \
  --signing-key "/astria/key"  \
  --sequencer-chain-id {{sequencer_chain_id}} \
  --sequencer-url {{sequencer_rpc_url}} \
  --input "/astria/tempfile"

  # Can inspect the file by removing and looking in volume
  docker volume remove cli-test-withdrawals

  # Validate that the Celestia balance timed out
  echo "Checking for 10 seconds to ensure that Celestia balance does not update"
  for i in {1..10}
  do
    current_celestia_balance=$(just ibc-test get-celestia-balance)
    echo "check $i, balance: $current_celestia_balance, if failure balance: $expected_celestia_balance"
    if (( $expected_celestia_balance == $current_celestia_balance )); then
      echo "Celestia balance changed when timeout should have occured"
      exit 1
    else
      sleep 1
    fi
  done

  # check that evm balance was refunded
  # we have already looped many times so we shouldn't need to wait
  current_balance=$(just evm-get-balance {{evm_destination_address}})
  if (( $post_deposit_evm_balance == $current_balance )); then
    echo "EVM balance refunded correctly - expected: $post_deposit_evm_balance, actual: $current_balance"
  else
    echo "EVM balance not refunded correctly - expected: $post_deposit_evm_balance, actual: $current_balance"
    echo "EVM balance was not refunded; IBC transfer refund on timeout to Celestia failed"
    exit 1
  fi

bridge_address := "astria1d7zjjljc0dsmxa545xkpwxym86g8uvvwhtezcr"
sequencer_address := "astria1cewd7alwml4fhx3w3lxq3vgf20cqe0qm650fac"
sequencer_sudo_pkey := "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90"
sequencer_sudo_address := "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
compat_address := "astriacompat1cewd7alwml4fhx3w3lxq3vgf20cqe0qmdzxmvn"
celestia_dev_account_address := "celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96"
sequencer_chain_id := "sequencer-test-chain-0"
celestia_chain_id := "celestia-local-0"
celestia_node_url := "http://rpc.app.celestia.localdev.me:80"
sequencer_tia_bridge_priv_key := "6015fbe1c365d3c5ef92dc891db8c5bb26ad454bec2db4762b96e9f8b2430285"
keyring_backend := "test"
celestia_desitnation_address := "0x4a58639fb5458e65e4fa917ff951c390292c24a1"
sequencer_rpc_url := "http://rpc.sequencer.localdev.me"
cli_image := "ghcr.io/astriaorg/astria-cli"

# This is the same address as used in deploy.just
evm_destination_address := "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"
evm_rpc_url := "http://executor.astria.localdev.me"
evm_withdraw_tx_hash := "0xaeacd44698269bd7c7121bfb0d55ddddea8b7cf66ca34d878e0f0f9ece18f48e"
# evm_withdraw_tx_raw is the tx to the withdraw smart contract on the evm.
# Uses private key for 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30 to sign tx.
# was created via:
#  `forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript ... --sig "withdrawToIbcChain()"`
# w/ values:
#  PRIVATE_KEY=0x8b3a7999072c9c9314c084044fe705db11714c6c4ed7cddb64da18ea270dd203
#  ASTRIA_WITHDRAWER=0xA58639fB5458e65E4fA917FF951C390292C24A15
#  ORIGIN_DESTINATION_CHAIN_ADDRESS=celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96
#  AMOUNT=1000000000000
evm_withdraw_tx_raw := "0xf9012f80843b9aca0782aa0a94a58639fb5458e65e4fa917ff951c390292c24a1585e8d4a51000b8c4a996e020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000002f63656c6573746961316d306b73646a6c3270356e7a68717933703437666b737635326174336c6e38383578766c393600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000820a95a0989411b03c5f6a909b56b9d555def3159e741ebe09ff5e0ee246560be11246f0a03ec88359eeb398be2b3ddb9b6dadde7869be9e2792fc413900407b7fa5bf6676"

# all in units of utia
transfer_amount := "53000"
transfer_fees := "26000"

# TODO: move this to deploy.just so that defaultNamespace need not be redefined
defaultNamespace := "astria-dev-cluster"
[no-cd]
_do-rollup-ibc-transfer namespace=defaultNamespace:
  echo "Performing IBC transfer..."
  kubectl exec -n {{namespace}} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd tx ibc-transfer transfer \
    transfer \
    channel-0 \
    {{bridge_address}} \
    "{{transfer_amount}}utia" \
    --memo="{\"rollupDepositAddress\":\"{{evm_destination_address}}\"}" \
    --chain-id="{{celestia_chain_id}}" \
    --from="{{celestia_dev_account_address}}" \
    --fees="{{transfer_fees}}utia" \
    --yes \
    --log_level=debug \
    --home /home/celestia \
    --keyring-backend="{{keyring_backend}}"'

_do-ibc-transfer namespace=defaultNamespace toAddress="":
  echo "Performing IBC transfer with compat address..."
  kubectl exec -n {{namespace}} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd tx ibc-transfer transfer \
    transfer \
    channel-0 \
    {{toAddress}} \
    "{{transfer_amount}}utia" \
    --chain-id="{{celestia_chain_id}}" \
    --from="{{celestia_dev_account_address}}" \
    --fees="{{transfer_fees}}utia" \
    --yes \
    --log_level=debug \
    --home /home/celestia \
    --keyring-backend="{{keyring_backend}}"'

get-celestia-balance address=celestia_dev_account_address namespace=defaultNamespace:
  #!/usr/bin/env bash
  balance=$(kubectl exec -n {{namespace}} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd query bank balances --denom utia --output json {{address}}' | jq '.amount')
  # remove quotes
  balance=${balance//\"/}
  echo $balance

get-celestia-height namespace=defaultNamespace:
  #!/usr/bin/env bash
  height=$(kubectl exec -n {{namespace}} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd query block' | jq '.block.header.height')
  # remove quotes
  height=${height//\"/}
  echo $height

# helper command to call withdraw smart contract, in aid of getting its raw bytes for the tests.
_forge-ibc-withdraw:
  #!/usr/bin/env bash
  # This is a helper script to call the withdraw contract on the evm, b/c we want its raw bytes.
  #  Using the tx's raw bytes allows us to avoid the dependency on forge in ci/cd.
  #
  # NOTE - There is a .env in ../crates/astria-bridge-contracts/astria-bridge-contracts that needs to be populated correctly.
  #
  # NOTE - to get raw bytes, copy the success hash from the output, then:
  #  e.g. `just evm-get-raw-transaction 0x3f1a45c167fd147e92c8c9e0570eff7c4fbe7dc5ca7ce2a52a734f91cd6f72c3`
  # This script called from `/charts/deploy.just, so cd'ing to contract directory
  cd ../crates/astria-bridge-contracts/astria-bridge-contracts
  pwd
  echo "Calling withdraw contract using forge"
  forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
    --rpc-url {{evm_rpc_url}} \
    --legacy \
    --broadcast \
    --sig "withdrawToIbcChain()" -vvvv

# FIXME - delete after development
cb address=celestia_dev_account_address namespace=defaultNamespace:
  kubectl exec -n {{namespace}} pods/celestia-local-0 celestia-app -- /bin/bash -c \
  'celestia-appd query bank balances {{address}}'

# init sequencer bridge account
init-bridge-acct tag=defaultTag:
  just init-ibc-bridge {{sequencer_tia_bridge_priv_key}} transfer/channel-0/utia nria {{tag}}
