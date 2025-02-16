_default:
  @just --list auctioneer-test

defaultTag := ""
defaultNamespace := "astria-dev-cluster"
evm_destination_address := "0x507056D8174aC4fb40Ec51578d18F853dFe000B2"
smoke_test_image := "bharath123456/auctioneer-smoke-test:latest"
sequencer_grpc_url := "grpc.sequencer.localdev.me"
eth_rpc_url := "http://executor.astria.localdev.me"
searcher_private_key := "5478c1df1db4867e4d29ab1cf6c6ed0ae0b2c3fbe7fc6e00edd0891d507d43f2"
rollup_name := "astria"
amount_to_send := "10000"

delete:
  -just delete celestia-local
  -just delete sequencers
  -just delete rollup

@deploy tag=defaultTag:
  echo "Deploying ingress controller..." && just deploy-ingress-controller > /dev/null
  just wait-for-ingress-controller > /dev/null
  echo "Deploying local celestia instance..."
  helm install celestia-local-chart ./celestia-local --namespace {{defaultNamespace}} --set fast=true --create-namespace  > /dev/null
  helm dependency update ./sequencer > /dev/null
  helm dependency update ./evm-stack > /dev/null

  echo "Setting up 3 astria sequencer nodes"
  echo "Setting up the first astria sequencers..." && helm install \
    -n astria-validator-node0 node0-sequencer-chart ./sequencer \
    -f ../dev/values/validators/all.yml \
    -f ../dev/values/validators/node0.yml \
    --set sequencer.optimisticBlockApis.enabled=true \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null

  echo "Setting up the second astria sequencer..." && helm install \
    -n astria-validator-node1 node1-sequencer-chart ./sequencer \
    -f ../dev/values/validators/all.yml \
    -f ../dev/values/validators/node1.yml \
    --set sequencer.optimisticBlockApis.enabled=true \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null

  echo "Setting up the third astria sequencer..." && helm install \
    -n astria-validator-node2 node2-sequencer-chart ./sequencer \
    -f ../dev/values/validators/all.yml \
    -f ../dev/values/validators/node2.yml \
    --set sequencer.optimisticBlockApis.enabled=true \
    {{ if tag != '' { replace('--set images.sequencer.devTag=# --set sequencer-relayer.images.sequencerRelayer.devTag=#', '#', tag) } else { '' } }} \
    --create-namespace > /dev/null

  just wait-for-sequencer > /dev/null
  echo "Starting Flame EVM rollup and the Auctioneer..." && helm install -n astria-dev-cluster astria-chain-chart ./evm-stack \
    -f ../dev/values/rollup/auctioneer-test.yaml \
    {{ if tag != '' { replace('--set evm-rollup.images.conductor.devTag=# --set auctioneer.images.auctioneer.devTag=# --set composer.images.composer.devTag=# --set evm-bridge-withdrawer.images.evmBridgeWithdrawer.devTag=#', '#', tag) } else { '' } }} \
    --set blockscout-stack.enabled=false \
    --set postgresql.enabled=false \
    --set auctioneer.enabled=true \
    --set config.geth.auctioneer=true \
    --set evm-bridge-withdrawer.enabled=false \
    --set evm-faucet.enabled=false > /dev/null
  # TODO - don't wait for evm-bridge-withdrawer
  just wait-for-rollup > /dev/null

[no-cd]
run tag=defaultTag:
  #!/usr/bin/env bash
  set +e
  echo "Running auctioneer smoke test 5 times..."
  echo

  success=0
  for i in {1..5} ; do
    echo "Running test iteration $i..."
    echo

    echo "Getting initial balance of {{evm_destination_address}}"
    initial_balance=$(just evm-get-balance {{evm_destination_address}})
    echo "Initial balance: $initial_balance"
    echo

    expected_balance=$(echo "$initial_balance + {{amount_to_send}}" | bc)

    docker run --rm \
    --network host {{smoke_test_image}} \
    --sequencer-url {{sequencer_grpc_url}} \
    --eth-rpc-url {{eth_rpc_url}} \
    --searcher-private-key {{searcher_private_key}} \
    --rollup-name {{rollup_name}} \
    --address-to-send {{evm_destination_address}} \
    --amount-to-send {{amount_to_send}} \

    if [ $? -ne 0 ]; then
      echo
      echo "Smoke test iteration $i failed!"
      echo
      continue
    fi

    echo
    echo "Getting final balance of {{evm_destination_address}}"
    final_balance=$(just evm-get-balance {{evm_destination_address}})

    echo "Verifying if the final balance is as expected..."
      if [ "$final_balance" -eq "$expected_balance" ]; then
          (( success++ ))
          echo "Smoke test iteration $i passed!"
      else
          echo "Smoke test iteration $i failed!"
          echo "Expected balance: $expected_balance, Actual balance: $final_balance"
      fi
     echo
  done
  echo "Done with all iterations!"
  echo
  echo "Verifying if the smoke test passed at least 3 times..."
  echo
  echo "$success/5 tests has passed"
  if [ "$success" -gt 2 ]; then
    echo "Smoke test passed!"
  else
    echo "Smoke test failed!"
    exit 1
  fi