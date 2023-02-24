# sequencer-relayer [wip]

This repo contains a functionality for relaying blocks from the Astria shared sequencer to a DA layer (ie. Celestia). 

Components:
- sequencer RPC client (`src/sequencer.rs`) which polls for latest blocks from a sequencer node
- Celestia RPC client (TODO) which writes sequencer blocks to Celestia
- command-line interface (TODO) for running the relayer

## Requirements

- Rust 1.66
- Go 1.19 (for running [metro](https://github.com/histolabs/metro.git))

## Build

```
cargo build --release
```

## Test

Run [metro](https://github.com/histolabs/metro.git):
```
git clone https://github.com/histolabs/metro.git
cd metro
make install
bash scripts/single-node.sh
```

Then, you can run the unit tests:
```
cargo test
```