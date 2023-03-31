# sequencer-relayer [wip]

This repo contains a functionality for relaying blocks from the Astria shared sequencer to a DA layer (ie. Celestia). 

Components:
- sequencer RPC client which polls for latest blocks from a sequencer node
- Celestia RPC client which writes sequencer blocks to Celestia
- command-line interface for running the relayer

## Requirements

- Rust 1.66
- Go 1.19 (for running [metro](https://github.com/astriaorg/metro.git))

## Build

```bash
cargo build --release
```

## Test

Run [metro](https://github.com/astriaorg/metro.git):
```bash
git clone https://github.com/astriaorg/metro.git
cd metro
git checkout noot/msg-type
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
cargo test -- --test-threads=1
```

## Run

While running Metro and Celestia, start the relayer:
```bash
./target/build/release/relayer
```

Note: the relayer automatically uses the validator private key located at `~/.metro/config/priv_validator_key.json`. You can specify the file with `-v`.

Then, submit a tx to Metro:
```bash
metro tx bank send validator metro1sdfn0kunm8yzm3rpxeqdcc0fk0ygw2lgggtnhp 300utick --keyring-backend="test" --fees 210utick --yes
```

You should see some logs such as:
```bash
Mar 03 14:17:21.432  INFO relayer: got block with height 82 from sequencer
Mar 03 14:17:22.561  INFO relayer: submitted block 82 to DA layer: tx count=1
```

Alternatively, you can use the small testing program [here](https://github.com/astriaorg/metro-transactions) to submit both "primary" and "secondary" transactions to Metro:
```bash
git clone https://github.com/astriaorg/metro-transactions
cd metro-transactions
go run main.go
```

## Running with Docker
```bash
# must map priv_validator_key.json to the container
docker run --rm -v ~/.metro/config/priv_validator_key.json:/root/.metro/config/priv_validator_key.json ghcr.io/astriaorg/sequencer-relayer:latest 
```