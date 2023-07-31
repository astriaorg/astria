use crate::{Composer, Config as ComposerConfig};

#[tokio::test]
async fn health_check_works() {
    let sample_config = ComposerConfig {
        log: "env=warn".into(),
        sequencer_url: "127.0.0.1:1210".parse().unwrap(),
        sequencer_address: "envaddress".to_string(),
        sequencer_secret: "envsecret".to_string(),
        api_port: 5050,
        chain_id: "envnet".to_string(),
        execution_ws_url: "127.0.0.1:40041".parse().unwrap(),
    };

    // Setting up composer with sample config
    let client = reqwest::Client::new();
    let composer = Composer::new(&sample_config).await;
    assert!(composer.is_err());
    let composer = composer.unwrap();
    let api_url = composer.api_listen_addr();

    // Starting the composer
    tokio::spawn(async move { composer.run_until_stopped() });

    // Sleep to ensure that the composer start-up task get priority in the executor and starts running
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    // Act
    let response = client
        .get(&format!("http://{}/healthz", api_url))
        .send()
        .await
        .expect("failed to send healthz request");

    // Assert
    assert!(response.status().is_success());
}
