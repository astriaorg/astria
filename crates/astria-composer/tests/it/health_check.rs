use tracing::info;

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let api_url = app.config.searcher.api_url;
    // TODO: test fails if i don't sleep here
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Act
    let response = client
        .get(&format!("http://{}/healthz", api_url))
        .send()
        .await
        .expect("failed to send healthz request");

    // Assert
    assert!(response.status().is_success());
}
