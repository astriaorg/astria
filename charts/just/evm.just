import 'defaults.just'


_default_evm:
  @just --list evm


#############################################
##    EVM Curl Command Helper Functions    ##
#############################################


# Get Deployed Contract Code
#############################
[doc("
Obtains the code of a deployed contract on the EVM given its address.
Usage:
  just evm get-deployed-contract-code [CONTRACT_ADDRESS] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-deployed-contract-code contractAddress rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_getCode","params":["{{contractAddress}}", "latest"],"id":1}' -H 'Content-Type: application/json' | jq -r '.result'


# Get Transaction Receipt
##########################
[doc("
Obtains the receipt of a transaction on the EVM given its hash.
Usage:
  just evm get-transaction-receipt [TX_HASH] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-transaction-receipt txHash rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_getTransactionReceipt","params":["{{txHash}}"],"id":1}' -H 'Content-Type: application/json' | jq '.result'


# Get Raw Transaction
######################
[doc("
Obtains the raw transaction on the EVM given its hash.
Usage:
  just evm get-raw-transaction [TX_HASH] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-raw-transaction txHash rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_getRawTransactionByHash","params":["{{txHash}}"],"id":1}' -H 'Content-Type: application/json' | jq '.result'


# Send Raw Transaction
#######################
[doc("
Sends a raw transaction to the EVM.
Usage:
  just evm send-raw-transaction [TX_DATA] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
send-raw-transaction txData rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["{{txData}}"],"id":1}' -H 'Content-Type: application/json'


# Get Block By Number
######################
[doc("
Obtains a block on the EVM given its number.
Usage:
  just evm get-block-by-number [BLOCK_NUMBER] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-block-by-number blockNumber rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["{{blockNumber}}", false],"id":1}' -H 'Content-Type: application/json' | jq '.result'


# Get Balance
##############
[doc("
Obtains the balance of an address on the EVM.
Usage:
  just evm get-balance [ADDRESS] <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-balance address rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  HEX_NUM=$(curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["{{address}}", "latest"],"id":1}' -H 'Content-Type: application/json' | jq -r '.result')
  echo $(just evm::hex-to-dec $HEX_NUM)


# Get Transaction Pool Length
##############################
[doc("
Obtains the length of the transaction pool on the EVM.
Usage:
  just evm get-txpool-length <RPC_URL>
    (see 'defaults.just' for default 'eth_rpc_url')
")]
get-txpool-length rpcUrl=eth_rpc_url:
  #!/usr/bin/env bash
  curl -X POST {{eth_rpc_url}} -s -d '{"jsonrpc":"2.0","method":"txpool_content","params":[],"id":1}' -H 'Content-Type: application/json' | jq '.result'


[private]
hex-to-dec hex:
  #!/usr/bin/env bash
  HEX_NUM={{hex}}
  CLEAN_HEX=$(echo "${HEX_NUM#0x}" | tr 'a-z' 'A-Z')
  echo $(echo "ibase=16; $CLEAN_HEX" | bc)
