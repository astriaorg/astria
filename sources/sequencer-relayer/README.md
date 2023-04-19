# sequencer-relayer [wip]

This repo contains a functionality for relaying blocks from the Astria shared sequencer to a DA layer (ie. Celestia). 

Components:
- sequencer RPC client which polls for latest blocks from a sequencer node
- Celestia RPC client which writes sequencer blocks to Celestia
- command-line interface for running the relayer

## Requirements

- Rust 1.66
- Podman 4 (for running tests)

## Build

```sh
$ cargo build --bin relayer --release
```

## Test

To run integration tests a (rootless) podman API service must be present.
Currently, only macOS and Linux are supported. Installation instructions
are available on [podman.io](https://podman.io/getting-started/installation.html).

After following the steps for MacOS, the podman API service should be immediately
available. On linux, one might need to issue:
```sh
$ systemctl --user enable --now podman.socket
```
Because podman starts a pod of several containers, it is advisable to make a
dry-run for podman to pull all images first:
```sh
$ podman run \
  -e pod_name=sequencer_relayer_stack \
  -e celestia_home_volume=celestia-home-vol \
  -e metro_home_volume=metro-home-vol \
  -e bridge_host_port=26659 \
  -e sequencer_host_port=1318 \
  -e scripts_host_volume=$PWD/containers \
  -v $PWD/templates:/data/templates \
  bbcrd/j2cli:latest \
  -o templates/sequencer_relayer_stack.yaml \
  templates/sequencer_relayer_stack.yaml.jinja2

$ podman kube play --start=false templates/sequencer_relayer_stack.yaml
```
Tests can then be run with
```sh
$ cargo test
```
Note: because most tests require that blocks be available on the data
availability layer (celestia) or in the sequencer (metro), integration tests
currently have very long `sleep` steps, waiting for 30 seconds and more to
ensure that blocks have been commited prior to executing API calls. To
run onl library unit tests and skip integration tests run:
```sh
$ cargo test --lib
```

### Troubleshooting

Sometimes pods don't get stopped and cleaned up after tests have finished running.
This is frequently related to ports in the range 1024 to 65536 not being available.
You will then see errors like these:
```
failed playing YAML failed on podman:
Fault {
  code: 500,
  message: "playing YAML file: starting some containers: internal libpod error"
}  
```
Try stopping all pods and rerunning the command:
```sh
$ podman pod stop --all && podman pod rm --all
$ cargo test
```

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