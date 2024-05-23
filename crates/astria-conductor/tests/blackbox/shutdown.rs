use astria_conductor::config::CommitLevel;

use crate::helpers::spawn_conductor;

/// Only tests if conductor shuts down during bringup.
///
/// No mocks are mounted so that all consituent Conductor tasks
/// should be stuck in their runtime-initialization phases (if any).
/// Once `TestConductor`'s `Drop` implementation runs, its shutdown
/// logic is invoked.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn shut_down_during_bringup() {
    spawn_conductor(CommitLevel::SoftAndFirm).await;
}
