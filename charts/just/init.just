import 'defaults.just'


_default_init:
    @just --list init


############################
## Initialization Recipes ##
############################


# Initialize Rollup Bridge
###########################
[doc("
Initializes a bridge account for the given rollup on the Sequencer chain.
Usage:
  $just init rollup-bridge <TAG> <ROLLUP_NAME> <EVM_DESTINATION_ADDRESS> <TRANSFER_AMOUNT>
    defaults:
      TAG: ''
      ROLLUP_NAME: 'astria'
      EVM_DESTINATION_ADDRESS: evm_destination_address (see 'defaults.just')
      TRANSFER_AMOUNT: '10'
")]
rollup-bridge tag=defaultTag rollupName=defaultRollupName evmDestinationAddress=evm_destination_address transferAmount=sequencer_transfer_amount:
  #!/usr/bin/env bash
  set -e

  SEQUENCER_FUNDS_PKEY="934ab488f9e1900f6a08f50605ce1409ca9d95ebdc400dafc2e8a4306419fd52"
  ASSET="nria"
  FEE_ASSET="nria"
  TRANSFER_AMOUNT=$(echo "{{transferAmount}} * {{sequencer_base_amount}}" | bc)

  IMAGE="{{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }}"

  docker run --rm --network host $IMAGE sequencer init-bridge-account \
    --rollup-name {{rollupName}} \
    --private-key {{sequencer_bridge_pkey}} \
    --sequencer.chain-id {{sequencer_chain_id}} \
    --sequencer-url {{sequencer_rpc_url}} \
    --fee-asset=$FEE_ASSET --asset=$ASSET || exit 1
  docker run --rm --network host $IMAGE sequencer bridge-lock {{sequencer_bridge_address}} \
    --amount $TRANSFER_AMOUNT \
    --destination-chain-address {{evmDestinationAddress}} \
    --private-key $SEQUENCER_FUNDS_PKEY \
    --sequencer.chain-id {{sequencer_chain_id}} \
    --sequencer-url {{sequencer_rpc_url}} \
    --fee-asset=$FEE_ASSET --asset=$ASSET


# Initialize IBC Bridge
########################
[doc("
Initializes an IBC bridge account for the given rollup on the Sequencer chain.
Usage:
  just init ibc-bridge [privateKey] [asset] [feeAsset] <TAG> <ROLLUP_NAME>
      defaults:
        TAG: ''
        ROLLUP_NAME: 'astria'
")]
ibc-bridge privateKey asset feeAsset tag=defaultTag rollupName=defaultRollupName:
  @echo "Initializing IBC bridge accoung for rollup {{rollupName}} with asset {{asset}} \
    and feeAsset {{feeAsset}}..."
  @docker run --rm --network host {{cli_image}}{{ if tag != '' { replace(':#', '#', tag) } else { '' } }} sequencer init-bridge-account \
    --rollup-name {{ rollupName }} \
    --private-key {{ privateKey }} \
    --sequencer.chain-id {{ sequencer_chain_id }} \
    --sequencer-url {{ sequencer_rpc_url }} \
    --fee-asset {{ feeAsset }} \
    --asset {{ asset }}
