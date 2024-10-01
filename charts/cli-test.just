_default:
  @just --list cli-test

deploy:
  @just deploy astria-local

command tag *ARGS:
  docker run --rm --network host "ghcr.io/astriaorg/astria-cli{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}" {{ ARGS }}

fundedAccount := "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
sequencerRpc := "http://rpc.sequencer.localdev.me"
defaultTag := "latest"
run tag=defaultTag:
  #!/usr/bin/env bash
  set -e

  validate_output() {
    echo "$output"
    if echo "$output" | perl -0777 -ne 'exit 0 if /'"$expected_format"'/; exit 1'; then
      echo "Output matches the expected format."
    else
      echo "Output does not match the expected format."
      exit 1
    fi
  }

  printf "Testing the CLI with the tag: {{ tag }}\n"


  printf "\nRunning the account create command...\n"
  output=$(just cli-test command {{ tag }} sequencer account create)
  expected_format="^Create Sequencer Account\n\nPrivate Key: \"[0-9a-f]{64}\"\nPublic Key:  \"[0-9a-f]{64}\"\nAddress:     \"[0-9a-f]{40}\"$"
  validate_output

  printf "\nRunning the address bech32 command...\n"
  address=$(echo "$output" | perl -ne 'print $1 if /Address:\s+"([0-9a-f]{40})"/')
  # Run the second command with the extracted address
  output=$(just cli-test command {{ tag }} sequencer address bech32m --bytes "$address")
  expected_format="^astria[0-9a-z]{39}$"
  validate_output
  astria_address=$(echo "$output")
  
  printf "\nRunning the account balance command...\n"
  output=$(just cli-test command {{ tag }} sequencer account balance {{fundedAccount}} --sequencer-url {{sequencerRpc}})
  expected_format="^Balances for address: astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\n    333333333333333333 nria$"
  validate_output

  printf "\nRunning the account nonce command...\n"
  output=$(just cli-test command {{ tag }} sequencer account nonce {{fundedAccount}} --sequencer-url {{sequencerRpc}})
  expected_format="^Nonce for address astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm\n    0 at height [0-9]*$"
  validate_output
  
  printf "\nRunning the transfer command...\n"
  output=$(just cli-test command {{ tag }} sequencer transfer {{astria_address}} 1000000000000000000 --sequencer-url {{sequencerRpc}})