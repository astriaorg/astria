# sequencer-relayer

This repo contains a functionality for relaying blocks from the Astria shared sequencer to a DA layer (ie. Celestia). 

Components:
- sequencer RPC client which polls for latest blocks from a sequencer node
- Celestia RPC client which writes sequencer blocks to Celestia
- command-line interface for running the relayer

## Requirements

- Rust 1.66
- kind 0.18.0 (for tests)
- kubectl 1.26.3 (for tests)

## Building

```sh
$ cargo build --bin relayer --release
```

## Testing

See [`TESTING.md`](./TESTING.md).

## Run

With astria-sequencer/cometbft and Celestia running locally (see below), start the relayer with
While running the sequencer and Celestia, start the relayer:
```sh
cargo run --release
# Or after having built it
./target/release/astria-sequencer-relayer --validator-key-file=$HOME/.cometbft/config/priv_validator_key.json
```

The celestia cluster can be started by running the following from the root of the monorepo:
```sh
just create-cluster
just deploy-ingress-controller
just wait-for-ingress-controller
just start-celestia-jsonrpc-test-deployment
just wait-for-celestia-jsonrpc-test-deployment
```

Alternatively, you can disable writing to Celestia and only publish blocks via gossip:
```sh
./target/release/relayer --disable-writing
```

The relayer automatically listens on `/ip4/127.0.0.1/tcp/33900` and is also able to discover local peers via mDNS.
