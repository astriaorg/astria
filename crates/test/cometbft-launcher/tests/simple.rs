use std::time::Duration;

use cometbft_launcher::CometBft;

#[tokio::test]
async fn cometbft_is_launched_and_responds() {
    let cometbft = CometBft::builder()
        .proxy_app("noop")
        .launch()
        .await
        .expect("should be able to start cometbft if installed");
    let cometbft_rpc = cometbft.rpc_listen_addr;
    let client = reqwest::Client::new();
    let rsp = client
        .get(format!("http://{cometbft_rpc}/status"))
        .timeout(Duration::from_millis(500))
        .send()
        .await
        .expect("if cometbft launched it should respond to status requests");
    assert!(rsp.status().is_success());
}
