###########################
## Shared Default Values ##
###########################

defaultNamespace  := "astria-dev-cluster"
defaultRollupName := "astria"
defaultPrivateKey := ""
defaultNetworkId  := ""
defaultTag        := ""
defaultDockerTag  := "local"
defaultUpgradeName := "aspen"

defaultHermes := "local"

defaultSequencerStartBlock := ""

defaultHypAgentConfig         := ""
defaultHypRelayerPrivateKey   := ""
defaultHypValidatorPrivateKey := ""

evm_destination_address := "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"
eth_rpc_url := "http://executor.astria.127.0.0.1.nip.io/"

validatorName             := "single"
# 1 RIA is 10^9 nRIA
sequencer_base_amount     := "1000000000"
# 10 RIA
sequencer_transfer_amount := "10"
sequencer_rpc_url         := "http://rpc.sequencer.127.0.0.1.nip.io"
sequencer_bridge_address  := "astria13ahqz4pjqfmynk9ylrqv4fwe4957x2p0h5782u"
sequencer_bridge_pkey     := "dfa7108e38ab71f89f356c72afc38600d5758f11a8c337164713e4471411d2e0"
sequencer_chain_id        := "sequencer-test-chain-0"

cli_image := "ghcr.io/astriaorg/astria-cli"
