use crate::helper::spawn_composer;
#[tokio::test]
async fn readyz_with_one_rollup() {
    // spawn_composer hits `/readyz` as part of starting the test
    // environment. If this future return then `readyz` must have
    // returned `status: ok`.
    let _test_composer = spawn_composer(&["test1"]).await;
}

#[tokio::test]
async fn readyz_with_two_rollups() {
    // spawn_composer hits `/readyz` as part of starting the test
    // environment. If this future return then `readyz` must have
    // returned `status: ok`. blah
    let _test_composer = spawn_composer(&["test1", "test2"]).await;
}
