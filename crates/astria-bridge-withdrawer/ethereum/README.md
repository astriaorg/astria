# astria-bridge-withdrawer

Forge project for the bridge withdrawer contract.

Requirements:

- foundry

Build:

```sh
forge build
```

Copy the example .env: `cp local.example.env .env`

Put your private key in `.env` and `source .env`.

Deploy `AstriaWithdrawer.sol`:

```sh
forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
   --rpc-url $RPC_URL --broadcast --sig "deploy()" -vvvv 
```

Call `withdrawToSequencer` in `AstriaWithdrawer.sol`:

```sh
forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
   --rpc-url $RPC_URL --broadcast --sig "withdrawToSequencer()" -vvvv
```

Call `withdrawToOriginChain` in `AstriaWithdrawer.sol`:

```sh
forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
   --rpc-url $RPC_URL --broadcast --sig "withdrawToOriginChain()" -vvvv
```

## Updating Smoke Test

If you change the contract you will need to update the configuration for the
smoke test. To do this, you must update the genesis contract in
`[repo-root]/dev/values/rollup/dev.yml`.

Note requires the [astria-go cli](https://github.com/astriaorg/astria-cli-go/?tab=readme-ov-file#installation)
installed.

1. First comment out the old genesis contract in the `genesisAlloc` section.
1. Deploy a new cluster:

    ```sh
    # If don't have a local cluster running
    > just deploy cluster
    > just deploy ingress-controller
    
    # Deploy astria components
    > just deploy astria-local
    
    # Deploy rollup, and init with funds
    > just deploy dev-rollup
    > just init rollup-bridge
    ```

1. Deploy the withdrawer contract, copy the success contract address:

    ```sh
    > cp cluster.env.example .env && source .env
    > forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
        --rpc-url $RPC_URL \
        --priority-gas-price 1 \
        --broadcast \
        --sig "deploy()" -vvvv
    ```

1. Get the contract address deployed:

    ```sh
    > just evm-get-deployed-contract-code <deployed-contract-address>
    <new-contract-code>
    ```

1. Update the `genesisAlloc` section in `[repo-root]/dev/values/rollup/dev.yml`
with the new contract code.
1. Submit a withdraw TX to the new contract:

    ```sh
    > forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
        --rpc-url $RPC_URL \
        --priority-gas-price 1 \
        --broadcast \
        --sig "withdrawToSequencer()" -vvvv 
    ```

1. Note the withdraw TX hash and get the raw tx code for it:

    ```sh
    > just evm-get-raw-transaction <withdraw-tx-hash>
    <new-bridge-tx>
    ```

1. Update the `bridge_tx_bytes` and `bridge_tx_hash` fields in
`[repo-root]/charts/deploy.just` with the new raw tx bytes and hash repectfully.
