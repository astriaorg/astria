import '../defaults.just'
import '../evm.just'

[doc("Run recipes for IBC testing. Use 'just --list run ibc-test' for more info.")]
mod ibc-test


# Set the working directory for all recipes to the project's root.
set working-directory := "../../../"


_default_run:
  @just --list run


#################
## Run Recipes ##
#################


# RIA is 10^9, WEI is 10^18, 10^9 * 10^9 = 10^18
rollup_multiplier := "1000000000"
eth_ws_url := "ws://ws-executor.astria.localdev.me/"
# bridge_tx_bytes is the tx to the withdraw smart contract on the evm.
# Uses private key for 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30 to sign tx.
# was created via:
#  `forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
#        --rpc-url "http://executor.astria.localdev.me/" \
#        --legacy \
#        --broadcast \
#        --sig "withdrawToSequencer()" -vvvv`
# w/ values:
#  PRIVATE_KEY=0x8b3a7999072c9c9314c084044fe705db11714c6c4ed7cddb64da18ea270dd203
#  ASTRIA_WITHDRAWER=0xA58639fB5458e65E4fA917FF951C390292C24A15
#  SEQUENCER_DESTINATION_CHAIN_ADDRESS="astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504"
#  AMOUNT=1000000000000000000
bridge_tx_bytes := "0xf8f280843ba60f5782a35194a58639fb5458e65e4fa917ff951c390292c24a15880de0b6b3a7640000b884bab916d00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002d617374726961313777306164656736346b7930646178776432756779756e65656c6c6d6a676e786c333935303400000000000000000000000000000000000000820a95a034652da1bbcad94f6af3db785127dae70f9b4e7d4da3c3f4b36eafe7fce9bf58a0169ed71974bcd74f0cea148148b5f3f8da50cdd055057dd18a599a2a3e14679f"
bridge_tx_hash := "0x326c3910da4c96c5a40ba1505fc338164b659729f2f975ccb07e8794c96b66f6"


# Run Smoke Test
#################
[doc("
Runs end-to-end test of Astria stack and EVM rollup, including bridge in/out and
TX finalization.
Usage:
  just run smoke-test <TAG> (default: '')
")]
smoke-test tag=defaultTag:
  #!/usr/bin/env bash
  set -e
  ASTRIA_CLI_IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"
  MAX_CHECKS=30

  # Checking starting balance
  BALANCE=$(just evm get-balance {{evm_destination_address}})
  if [ $BALANCE -ne 0 ]; then
    echo "Starting balance is not correct"
    exit 1
  fi

  echo "Testing Bridge In..."
  just init rollup-bridge {{tag}}
  CHECKS=0
  EXPECTED_BALANCE=$(echo "{{sequencer_transfer_amount}} * {{sequencer_base_amount}} * {{rollup_multiplier}}" | bc)
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(just evm get-balance {{evm_destination_address}})
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge In success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge In failure"
    exit 1
  fi

  echo "Testing Bridge Out..."
  just evm send-raw-transaction {{bridge_tx_bytes}}
  TRANSFERED_BALANCE=$(echo "1 * {{sequencer_base_amount}} * {{rollup_multiplier}}" | bc)
  EXPECTED_BALANCE=$(echo "$EXPECTED_BALANCE - $TRANSFERED_BALANCE" | bc)
  CHECKS=0
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(just evm get-balance {{evm_destination_address}})
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge Out EVM success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge Out EVM failure"
    exit 1
  fi
  CHECKS=0
  EXPECTED_BALANCE=$(echo "1 * {{sequencer_base_amount}}" | bc)
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer account balance astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504 --sequencer-url {{sequencer_rpc_url}}  | awk '/nria/{print $(NF-1)}')
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge Out Sequencer success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge Out Sequencer failure"
    exit 1
  fi

  echo "Testing TX Finalization"
  CHECKS=0
  BLOCK_NUM_HEX=$(just evm get-transaction-receipt {{bridge_tx_hash}} | jq -r '.blockNumber')
  BLOCK_NUM=$(just evm::hex-to-dec $BLOCK_NUM_HEX)
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    FINALIZED_BLOCK=$(just evm get-block-by-number finalized | jq -r '.number')
    FINALIZED_BLOCK=$(just evm::hex-to-dec $FINALIZED_BLOCK)
    echo "Check $CHECKS, Finalized block: $FINALIZED_BLOCK, Expected: $BLOCK_NUM"
    if [ $FINALIZED_BLOCK -ge $BLOCK_NUM ]; then
      echo "Finalization success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Finalization failure"
    exit 1
  fi
  exit 0


# Run Smoke Test CLI
#####################
evm_contract_address := "0xA58639fB5458e65E4fA917FF951C390292C24A15"
[doc("
Runs end-to-end smoke test of Astria stack (minus Bridge-Withdrawer) and EVM rollup,
including bridge in/out and TX finalization.
Usage:
  just run smoke-cli <TAG> (default: '')
")]
smoke-cli tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  ASTRIA_CLI_IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"
  MAX_CHECKS=30

  # Checking starting balance
  BALANCE=$(just evm get-balance {{evm_destination_address}})
  if [ $BALANCE -ne 0 ]; then
    echo "Starting balance is not correct"
    exit 1
  fi

  echo "Testing Bridge In..."
  just init rollup-bridge {{tag}}
  CHECKS=0
  EXPECTED_BALANCE=$(echo "{{sequencer_transfer_amount}} * {{sequencer_base_amount}} * {{rollup_multiplier}}" | bc)
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(just evm get-balance {{evm_destination_address}})
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge In success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge In failure"
    exit 1
  fi

  echo "Testing Bridge Out..."
  just evm send-raw-transaction {{bridge_tx_bytes}}
  TRANSFERED_BALANCE=$(echo "1 * {{sequencer_base_amount}} * {{rollup_multiplier}}" | bc)
  EXPECTED_BALANCE=$(echo "$EXPECTED_BALANCE - $TRANSFERED_BALANCE" | bc)
  CHECKS=0
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(just evm get-balance {{evm_destination_address}})
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge Out EVM success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge Out EVM failure"
    exit 1
  fi

  CURRENT_BLOCK_HEX=$(just evm get-block-by-number latest | jq -r '.number')
  CURRENT_BLOCK=$(just evm::hex-to-dec $CURRENT_BLOCK_HEX)

  # Using a docker volume to handle both passing in a private key & the output file
  docker volume create cli-test-withdrawals
  docker run --rm -v cli-test-withdrawals:/data alpine sh -c "echo '{{sequencer_bridge_pkey}}' > /data/key"
  docker run --rm -v cli-test-withdrawals:/astria --network host $ASTRIA_CLI_IMAGE bridge collect-withdrawals \
    --rollup-endpoint {{eth_ws_url}} \
    --contract-address {{evm_contract_address}} \
    --from-rollup-height 1 \
    --to-rollup-height $CURRENT_BLOCK \
    --sequencer-asset-to-withdraw nria \
    --bridge-address {{sequencer_bridge_address}} \
    --output "/astria/tempfile" \
    --force
   docker run --rm -v cli-test-withdrawals:/astria --network host $ASTRIA_CLI_IMAGE bridge submit-withdrawals \
    --signing-key "/astria/key"  \
    --sequencer-chain-id {{sequencer_chain_id}} \
    --sequencer-url {{sequencer_rpc_url}} \
    --input "/astria/tempfile"

  # Can inspect the file by removing and looking in volume
  docker volume remove cli-test-withdrawals

  CHECKS=0
  EXPECTED_BALANCE=$(echo "1 * {{sequencer_base_amount}}" | bc)
  while (( $CHECKS < $MAX_CHECKS )); do
    CHECKS=$((CHECKS+1))
    BALANCE=$(docker run --rm --network host $ASTRIA_CLI_IMAGE sequencer account balance astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504 --sequencer-url {{sequencer_rpc_url}}  | awk '/nria/{print $(NF-1)}')
    echo "Check $CHECKS, Balance: $BALANCE, Expected: $EXPECTED_BALANCE"
    if [ "$BALANCE" == "$EXPECTED_BALANCE" ]; then
      echo "Bridge Out Sequencer success"
      break
    else
      sleep 1
    fi
  done
  if (( $CHECKS >= $MAX_CHECKS )); then
    echo "Bridge Out Sequencer failure"
    exit 1
  fi

  exit 0


# Run Upgrade Test
###################
export PYTHONPATH := "${PYTHONPATH}:system-tests/helpers/generated"
[doc("
Runs the sequencer upgrade test in the `system-tests` folder.
<TAG> is the tag specifying the sequencer image to run to EXECUTE the upgrade,
not the image to run before the upgrade executes.
Usage:
  just run upgrade-test <TAG> (default: 'local')
")]
upgrade-test tag=defaultDockerTag:
  #!/usr/bin/env bash
  if [ -d "system-tests/.venv" ]; then
    echo "Found Python virtual environment, activating it..."
    source system-tests/.venv/bin/activate
    uv pip install -r system-tests/requirements.txt
  fi
  python3 system-tests/sequencer_upgrade_test.py \
    --image-tag {{ if tag != '' { tag } else { 'latest' } }} \
    --upgrade-name "aspen"
