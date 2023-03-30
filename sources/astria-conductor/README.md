# Astria Conductor

Coordinates blocks between the data layer and the execution layer.

### Running for development

* create `ConductorConfig.toml` in the repo root and populate accordingly, e.g.

```
celestia_node_url = "http://localhost:26659"
chain_id = "test"
rpc_address = "https://0.0.0.0:50051"
```

* run `cargo run`

### Tests

To run the tests, you need to build and run [`sequencer-relayer`](https://github.com/astriaorg/sequencer-relayer.git) as well as a Celestia cluster and Metro.

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

To run the relayer, inside `sequencer-relayer/`:
```bash
cargo build
./target/debug/relayer
```

Then, you can run the tests:
```bash
cargo test
```

### Run w/ Docker (wip):
```bash
# NOTE - there are currently issues with restarting containers, so ensure we start from a clean slate
./tests/docker/clean-docker.sh

# run the containers
docker-compose -f tests/docker/docker-compose.yml up

# run the tests
cargo test
```

Known issues:
* can't stop and restart bridge or metro container successfully
