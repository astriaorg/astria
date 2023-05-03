# Astria Conductor

Coordinates blocks between the data layer and the execution layer.

### Running for development

* create `ConductorConfig.toml` in the repo root and populate accordingly, e.g.

```
celestia_node_url = "http://localhost:26659"
tendermint_url = "http://localhost:1318"
chain_id = "ethereum"
execution_rpc_url = "http://localhost:50051"
```

* run `cargo run`

### Running dependencies with podman

First, ensure your local `ConductorConfig.toml` matches the values below.

Then you can run the following commands:

```bash
# create a local conductor_stack.yaml from the template
podman run --rm \
  -e pod_name=conductor_stack \
  -e celestia_home_volume=celestia-home-vol \
  -e metro_home_volume=metro-home-vol \
  -e executor_home_volume=executor-home-vol \
  -e relayer_home_volume=relayer-home-vol \
  -e executor_local_account=0xb0E31D878F49Ec0403A25944d6B1aE1bf05D17E1 \
  -e celestia_app_host_port=26657 \
  -e bridge_host_port=26659 \
  -e sequencer_host_port=1318 \
  -e sequencer_host_grpc_port=9100 \
  -e executor_host_http_port=8545 \
  -e executor_host_grpc_port=50051 \
  -e scripts_host_volume="$PWD"/container-scripts \
  -v "$PWD"/templates:/data/templates \
  dcagatay/j2cli:latest \
  -o /data/templates/conductor_stack.yaml \
  /data/templates/conductor_stack.yaml.jinja2

# play the pod with `kube play` which creates containers, pods, and volumes
podman kube play --log-level=debug templates/conductor_stack.yaml

# to run the conductor
cargo run
```

### Tests

Running all the tests will spin up a local Celestia cluster, Metro node, Geth node, and the relayer.
It will then run the tests against the local setup.

```bash
cargo test -- --nocapture --color always
```
