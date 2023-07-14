# Astria

Astria replaces centralized sequencers, allowing many rollups to share a single decentralized network of sequencers that’s simple and permissionless to join. This shared sequencer network provides out-of-the-box censorship resistance, fast block confirmations, and atomic cross-rollup composability – all while retaining each rollup’s sovereignty.

This repository contains the custom Astria components that make up the Astria network. Other components of the Astria network can be found in the [astriaorg](https://github.com/astriaorg) organization. 

To run locally, we utilize a dev-cluster which can be found at [astriaorg/dev-cluster](https://github.com/astriaorg/dev-cluster). 

To learn more about Astria, please visit [astria.org](https://astria.org).

## Components

* [conductor](https://github.com/astriaorg/astria/tree/main/crates/astria-conductor): conducts blocks from the data availability layer to the execution layer.
* [gossipnet](https://github.com/astriaorg/astria/tree/main/crates/astria-gossipnet): libp2p-based gossip network.
* [proto](https://github.com/astriaorg/astria/tree/main/crates/astria-proto): relevant protobufs for Astria types.
* [sequencer](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer): ABCI application that defines the sequencer state transition logic.
* [sequencer-relayer](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer-relayer): relays blocks from the sequencer chain to the data availability layer.

## Build

To build the relevant Astria binaries, you will need [Rust](https://www.rust-lang.org/tools/install) and [Buf](https://buf.build/docs/installation/) installed.

Then:
```sh
git clone https://github.com/astriaorg/astria.git
cd astria
cargo build --release
```

## Running locally

The entire stack consists of 6 different binaries. It's recommended to use the setup located in [astriaorg/dev-cluster](https://github.com/astriaorg/dev-cluster), but running everything manually is documented here as well.

To run the entire stack locally, you will additionally need cometbft installed, which requires that [Go](https://go.dev/doc/install) is installed.

The binaries required are as follows:
- astria-sequencer
- cometbft
- astria-sequencer-relayer
- Celestia
- astria-conductor
- Astria's go-ethereum fork

#### Install cometbft
Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your `PATH`.

```sh
git clone https://github.com/astriaorg/cometbft
cd cometbft
export GOPATH=~/go
make install
```

#### Start the sequencer chain

First, start `astria-sequencer`:

```sh
./target/debug/astria-sequencer --genesis-file=crates/astria-sequencer/test-genesis.json
```

Then, start cometbft:
```sh
cometbft init
cometbft start
```

#### Start Celestia (optional)

Note: this step is optional; we can configure the relayer and conductor to communicate directly.

You will need to have [kind/kubectl installed](https://kind.sigs.k8s.io/docs/user/quick-start/).

The celestia cluster can be started by running the following from the root of the monorepo:
```sh
just create-cluster
just deploy-ingress-controller
just wait-for-ingress-controller
just start-celestia-jsonrpc-test-deployment
just wait-for-celestia-jsonrpc-test-deployment
```

Then, get the Celestia JSON-RPC API token as follows:
```sh
# list pods
kubectl get -n astria-celestia-jsonrpc-client-test pods
# replace pod name with name printed above
kubectl exec -n astria-celestia-jsonrpc-client-test pods/<your-pod-name-with-hashes-etc> -c celestia-bridge -- cat /home/celestia/.admin_token
```

Take note of this token, as the relayer step requires it.

#### Start the relayer

Pass your token from above to the `--celestia-bearer-token` flag.

```sh
./target/release/astria-sequencer-relayer --celestia-bearer-token=<token-from-above-step> --validator-key-file=$HOME/.cometbft/config/priv_validator_key.json 
```

If Celestia is not running, pass the `--disable-writing` flag:

```sh
./target/release/astria-sequencer-relayer --celestia-bearer-token=<token-from-above-step>  --validator-key-file=$HOME/.cometbft/config/priv_validator_key.json --disable-writing
```

#### Build and start astria go-ethereum

```sh
git clone https://github.com/astriaorg/go-ethereum.git
cd go-ethereum
make geth
./build/bin/geth --datadir ~/.astriageth/ init genesis.json
./build/bin/geth --datadir ~/.astriageth/ --http --http.port=8545 --ws --ws.port=8545 --networkid=1337 --http.corsdomain='*' --ws.origins='*' --grpc --grpc.addr=localhost --grpc.port 50051
```

#### Start the conductor

```sh
./target/release/astria-conductor
```

If Celestia is not running, pass the `--disable-finalization` flag:

```sh
./target/release/astria-conductor --disable-finalization
```

#### Clean up local environment

Stop all running processes and 
```sh
pkill astria-sequencer-relayer && pkill astria-conductor && pkill geth && pkill astria-sequencer && pkill cometbft
rm -rf ~/.cometbft && rm -rf ~/.astriageth
just delete-cluster
```

## Testing

To run unit tests:
```sh
cargo test
```

## Contributing

Pull requests should be created against the `main` branch. In general, we follow the "fork-and-pull" Git workflow.

1. Fork the repo on GitHub
2. Clone the project to your own machine
3. Commit changes to your own branch
4. Push your work back up to your fork
5. Submit a Pull request so that we can review your changes

NOTE: Be sure to merge the latest from upstream before making a pull request!

## Issues

If you encounter any issues while using this project or have any questions, please open an issue in this repository [here](https://github.com/astriaorg/astria/issues).
