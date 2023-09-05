use cometbft_launcher::CometBft;

#[tokio::main]
async fn main() {
    let cometbft = CometBft::builder()
        .proxy_app("noop")
        .launch()
        .await
        .unwrap();
    let cometbft_rpc = cometbft.rpc_listen_addr;
    let status = reqwest::get(format!("http://{cometbft_rpc}/status"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("cometbft status response:\n{status}");
}
