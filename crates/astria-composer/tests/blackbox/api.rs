use crate::helper::spawn_composer;
#[tokio::test]
async fn readyz() {
    // spawn_composer hits `/readyz` as part of starting the test
    // environment. If this future return then `readyz` must have
    // returned `status: ok`.
    let _test_composer = spawn_composer().await;
}
