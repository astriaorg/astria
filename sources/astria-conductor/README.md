# Astria Conductor

Coordinates blocks between the data layer and the execution layer.

### Running for development

* create `ConductorConfig.toml` in the repo root and populate accordingly, e.g.

Note: I've been generating random namespace ids for development. [See how here](https://go.dev/play/p/7ltvaj8lhRl)

```
celestia_node_url = "http://localhost:26659"
namespace_id = "b860ccf0e97fdf6c"
rpc_address = "https://[::1]:50051"
```

* run `cargo run`
