# Launch cometbft in tests

This crate launches cometbft as a subprocess. It is intended to be used in tests.

Example usage:

```rust
use std::time::Duration;

use cometbft_launcher::CometBft;
use reqwest::Client;

#[tokio::main]
async fn main() {
    let cometbft = CometBft::builder()
        .proxy_app("noop")
        .launch()
        .await
        .expect("should be able to start cometbft if installed");
    let cometbft_rpc = cometbft.rpc_listen_addr;
    let client = Client::new();
    let rsp = client
        .get(format!("http://{cometbft_rpc}/status"))
        .timeout(Duration::from_millis(500))
        .send()
        .await
        .expect("if cometbft launched it should respond to status requests");
    assert!(rsp.status().is_success());
}
```
