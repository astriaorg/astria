# sequencer-relayer [wip]

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

With Metro and Celestia running locally (see below), start the relayer with
While running Metro and Celestia, start the relayer:
```sh
$ cargo run --bin relayer --release
# Or after having built it
$ ./target/release/relayer
```

The celestia cluster can be started by using the provided docker compose:
```sh
$ docker compose -f docker/test-docker-compose.yml up -d bridge0
```

Alternatively, you can disable writing to Celestia and only publish blocks via gossip:
```sh
$ ./target/release/relayer --disable-writing
```

The relayer automatically listens on `/ip4/127.0.0.1/tcp/33900` and is also able to discover local peers via mDNS.

Metro is compiled from source:
```sh
$ git clone https://github.com/astriaorg/metro.git
$ cd metro
$ git checkout noot/msg-type
$ make install
$ bash scripts/single-node.sh
```

Note: the relayer automatically uses the validator private key located at `~/.metro/config/priv_validator_key.json`. You can specify the file with `-v`.

Then, submit a tx to Metro:
```sh
$ metro tx bank send validator \
  metro1sdfn0kunm8yzm3rpxeqdcc0fk0ygw2lgggtnhp 300utick \
  --keyring-backend="test" \
  --fees 210utick \
  --yes
```

You should see some logs such as:
```sh
Mar 03 14:17:21.432  INFO relayer: got block with height 82 from sequencer
Mar 03 14:17:22.561  INFO relayer: submitted block 82 to DA layer: tx count=1
```

Alternatively, you can use the small testing program [here](https://github.com/astriaorg/metro-transactions) to submit both "primary" and "secondary" transactions to Metro:
```sh
$ git clone https://github.com/astriaorg/metro-transactions
$ cd metro-transactions
$ go run main.go
```

## Running with Docker
```sh
# must map priv_validator_key.json to the container
$ docker run --rm -v ~/.metro/config/priv_validator_key.json:/root/.metro/config/priv_validator_key.json ghcr.io/astriaorg/sequencer-relayer:latest 
```