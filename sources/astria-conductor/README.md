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


### Tests (with Docker):
* Optional: Make a new account in Metamask (or whichever method you prefer). Copy/paste the address into `tests/docker/.env` as `ACCOUNT_ID`. Otherwise, the default account id will be used.
  * This account will be allocated 300 ETH at startup.
```bash
# NOTE - there are currently issues with restarting containers, so ensure we start from a clean slate
./tests/docker/cleanup-docker.sh

# run the containers
docker-compose -f tests/docker/test-docker-compose.yml up -d   

# run the tests
cargo test

# cleanup the containers. 
# this is necessary to run fairly often because of issues with the 
# celestia image not handling restarts well.
./tests/docker/cleanup-docker.sh
```

### Running with Docker:
```bash
# NOTE - there are currently issues with restarting containers, so ensure we start from a clean slate
./tests/docker/cleanup-docker.sh

# run the containers
docker-compose -f tests/docker/test-docker-compose.yml up -d

# run the conductor
cargo run   

# follow a specific container's logs. -f is for follow, -t is for timestamps
docker logs -f -t geth0

# follow all container logs. You must specify the compose file if not ran from the same directory.
docker-compose -f tests/docker/test-docker-compose.yml logs --tail=0 -f -t
```

Known issues:
* can't stop and restart bridge or metro container successfully. You must use `./tests/docker/cleanup-docker.sh`

### Tests (old way without Docker. Using Docker is recommended.)

To run the tests, you need to build and run [`sequencer-relayer`](https://github.com/astriaorg/sequencer-relayer.git) as well as a Celestia cluster and Metro.

Run [metro](https://github.com/astriaorg/metro.git):
```bash
git clone https://github.com/astriaorg/metro.git
cd metro
git checkout astria
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
