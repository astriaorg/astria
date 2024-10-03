_default:
  @just --list

cli-test *ARGS:
  @just {{ARGS}}

deploy:
  @just deploy cluster
  @just deploy ingress-controller
  @just wait-for-ingress-controller
  @just deploy astria-local

command tag *ARGS:
  docker run --rm --network host "ghcr.io/astriaorg/astria-cli{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}" {{ ARGS }}

fundedAccount := "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
privKey1 := "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90"
privKey2 := "dfa7108e38ab71f89f356c72afc38600d5758f11a8c337164713e4471411d2e0"
ibcAddAddress := "astria13ahqz4pjqfmynk9ylrqv4fwe4957x2p0h5782u"
pubKey := "88787e29db8d5247c6adfac9909b56e6b2705c3120b2e3885e8ec8aa416a10f1"
newPower := "100"
originalPower := "10"
sequencerRpc := "http://rpc.sequencer.localdev.me"
sequencerChainId := "sequencer-test-chain-0"
asset := "test"
defaultTag := "latest"

validate-output output expected_format:
  #!/usr/bin/env bash
  set -e 
  
  if echo "$output" | perl -0777 -ne 'exit 0 if /'"$expected_format"'/; exit 1'; then
    printf "\e[32m✔\e[0m Output matches the expected format.\n"
  else
    printf "❌ Output does not match the expected format.\n"
    exit 1
  fi

# Test the `create-account` command
create-account tag=defaultTag:
  #!/usr/bin/env bash
  set -e 

  printf "\nRunning the 'sequencer account create' command...\n"
  output=$(just cli-test command {{ tag }} sequencer account create)
  expected_format="^Create Sequencer Account\n\nPrivate Key:\s+\"[0-9a-f]{64}\"\nPublic Key:\s+\"[0-9a-f]{64}\"\nAddress:\s+\"[0-9a-f]{40}\"$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

# Test the `bech32m` command
bech32m tag=defaultTag:
  #!/usr/bin/env bash
  set -e 

  # make a new account to get the address bytes
  output=$(just cli-test command {{ tag }} sequencer account create)
  expected_format="^Create Sequencer Account\n\nPrivate Key:\s+\"[0-9a-f]{64}\"\nPublic Key:\s+\"[0-9a-f]{64}\"\nAddress:\s+\"[0-9a-f]{40}\"$"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer address bech32' command...\n"
  address=$(echo "$output" | perl -ne 'print $1 if /Address:\s+"([0-9a-f]{40})"/')
  # Run the second command with the extracted address
  output=$(just cli-test command {{ tag }} sequencer address bech32m --bytes "$address")
  expected_format="^astria[0-9a-z]{39}$"
  astria_address=$(echo "$output")
  just cli-test validate-output "$astria_address" "$expected_format"

get-balance tag=defaultTag:
  #!/usr/bin/env bash
  set -e 

  printf "\nRunning the 'sequencer account balance' command...\n"
  output=$(just cli-test command {{ tag }} sequencer account balance {{fundedAccount}} --sequencer-url {{sequencerRpc}})
  expected_format="^Balances for address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\n\s+[0-9]{18} nria$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer balance get' command...\n"
  output=$(just cli-test command {{ tag }} sequencer balance get {{fundedAccount}} --sequencer-url {{sequencerRpc}})
  expected_format="^Balances for address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\n\s+[0-9]{18} nria$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

get-nonce tag=defaultTag:
  #!/usr/bin/env bash
  set -e 

  printf "\nRunning the 'sequencer account nonce' command...\n"
  output=$(just cli-test command {{ tag }} sequencer account nonce {{fundedAccount}} --sequencer-url {{sequencerRpc}})
  expected_format="^Nonce for address astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\n\s+[0-9]+ at height [0-9]*$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

get-blockheight tag=defaultTag:
  #!/usr/bin/env bash
  set -e 

  printf "\nRunning the 'sequencer blockheight get' command...\n"
  output=$(just cli-test command {{ tag }} sequencer blockheight get --sequencer-url {{sequencerRpc}})
  expected_format="^Block Height:\n\s+[0-9]*$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

ibc-relayer tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "\nRunning the 'sequencer sudo ibc-relayer add' command...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo ibc-relayer add --address {{ibcAddAddress}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nIbcRelayerChangeAction::Addition completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer sudo ibc-relayer remove' command...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo ibc-relayer remove --address {{ibcAddAddress}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nIbcRelayerChangeAction::Removal completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

fee-asset tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "\nRunning the 'sequencer sudo fee-asset add' command...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo fee-asset add --asset {{asset}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nFeeAssetChangeAction::Addition completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer sudo fee-asset remove' command...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo fee-asset remove --asset {{asset}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nFeeAssetChangeAction::Removal completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

sudo-address-change tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "\nRunning the 'sequencer sudo sudo-address-change' command...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo sudo-address-change --address {{ibcAddAddress}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nSudoAddressChange completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer sudo sudo-address-change' command to revert the sudo address to the original one...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo sudo-address-change --address {{fundedAccount}} --private-key {{privKey2}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria13ahqz4pjqfmynk9ylrqv4fwe4957x2p0h5782u\nSudoAddressChange completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

validator-update tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "\nRunning the 'sequencer sudo validator-update' command to update power...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo validator-update --power 10 --validator-public-key {{pubKey}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nValidatorUpdate completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

  printf "\nRunning the 'sequencer sudo validator-update' command to revert power...\n"
  output=$(just cli-test command {{ tag }} sequencer sudo validator-update --power 1 --validator-public-key {{pubKey}} --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nValidatorUpdate completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

transfer tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "\nRunning the 'sequencer transfer' command...\n"
  output=$(just cli-test command {{ tag }} sequencer transfer {{ibcAddAddress}} --amount 100 --private-key {{privKey1}} --sequencer-url {{sequencerRpc}} --sequencer.chain-id {{sequencerChainId}})
  expected_format="^sending tx from address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\nTransfer completed!\nIncluded in block: [0-9]+$"
  echo "Output:\n$output"
  just cli-test validate-output "$output" "$expected_format"

run tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  printf "Testing the CLI with the tag: {{ tag }}\n"

  just cli-test create-account
  just cli-test bech32m
  just cli-test get-balance
  just cli-test get-nonce
  just cli-test get-blockheight
  just cli-test ibc-relayer
  just cli-test fee-asset
  just cli-test sudo-address-change
  just cli-test validator-update
  just cli-test transfer
