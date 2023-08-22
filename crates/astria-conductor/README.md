# astria-conductor

Coordinates blocks between the data availability layer and the execution layer.

### Running for development

* create `ConductorConfig.toml` in the repo root and populate accordingly, e.g.

```
celestia_node_url = "http://localhost:26659"
tendermint_url = "http://localhost:1318"
chain_id = "ethereum"
execution_rpc_url = "http://localhost:50051"
```

* run `cargo run`

* alternatively, you can do `cargo build && ./target/debug/conductor`.

* to connect directly to a node via p2p, you can use the `--bootnodes` flag, e.g. `--bootnodes=/ip4/127.0.0.1/tcp/33900` or `--bootnodes=/ip4/127.0.0.1/tcp/34471/p2p/12D3KooWDCHwgGetpJuHknJqv2dNbYpe3LqgH8BKrsYHV9ALpAj8`.

### Tests

Running all the tests will spin up a local Celestia cluster, Metro node, Geth node, and the relayer.
It will then run the tests against the local setup.

```bash
cargo test -- --nocapture --color always
```
