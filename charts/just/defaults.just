###########################
## Shared Default Values ##
###########################

defaultNamespace  := "astria-dev-cluster"
defaultRollupName := "astria"
defaultPrivateKey := ""
defaultNetworkId  := ""
defaultTag        := ""
defaultDockerTag  := "local"

defaultSequencerStartBlock := ""

defaultWithdrawerAddress := ""

defaultHypAgentConfig         := ""
defaultHypRelayerPrivateKey   := ""
defaultHypValidatorPrivateKey := ""

evm_destination_address := "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"
frost_evm_destination_address := "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
frost_withdrawer_address := "astria1w2lxx9p02u7u934ljl060wannqm40g3y089lmh"
eth_rpc_url := "http://executor.astria.localdev.me/"

validatorName             := "single"
# 1 RIA is 10^9 nRIA
sequencer_base_amount     := "1000000000"
# 10 RIA
sequencer_transfer_amount := "10"
sequencer_rpc_url         := "http://rpc.sequencer.localdev.me"
sequencer_bridge_address  := "astria13ahqz4pjqfmynk9ylrqv4fwe4957x2p0h5782u"
sequencer_bridge_pkey     := "dfa7108e38ab71f89f356c72afc38600d5758f11a8c337164713e4471411d2e0"
sequencer_chain_id        := "sequencer-test-chain-0"

signerName := "signer1"

cli_image := "ghcr.io/astriaorg/astria-cli"
