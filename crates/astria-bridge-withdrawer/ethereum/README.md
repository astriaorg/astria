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
