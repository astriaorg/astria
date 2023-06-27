use crate::helpers::spawn_searcher;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let searcher = spawn_searcher().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("http://{}/healthz", searcher.inner.api_url()))
        .send()
        .await
        .expect("failed to send healthz request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
