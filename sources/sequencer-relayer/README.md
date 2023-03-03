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

```bash
cargo build --release
```

## Test

Run [metro](https://github.com/histolabs/metro.git):
```bash
git clone https://github.com/histolabs/metro.git
cd metro
make install
bash scripts/single-node.sh
```

Run a Celestia cluster:
```bash
git clone https://github.com/astriaorg/sequencer-relayer.git
cd sequencer-relayer
docker compose -f docker/test-docker-compose.yml up -d bridge0
```

Then, you can run the unit tests:
```bash
cargo test
```

## Run

While running Metro and Celestia, start the relayer:
```bash
./target/build/release/relayer 
```

Then, submit a tx to Metro:
```bash
metro tx bank send validator metro1sdfn0kunm8yzm3rpxeqdcc0fk0ygw2lgggtnhp 300utick --keyring-backend="test" --fees 210utick --yes
```

You should see some logs such as:
```bash
Mar 03 14:17:21.432  INFO relayer: got block with height 82 from sequencer
Mar 03 14:17:22.561  INFO relayer: submitted block 82 to DA layer: tx count=1
```
