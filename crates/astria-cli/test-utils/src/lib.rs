use std::{
    env,
    future::Future,
};

use once_cell::sync::Lazy;
use tempfile::TempDir;
use tokio::sync::Mutex as AsyncMutex;

static ASYNC_CURRENT_DIR_LOCK: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

/// Run an async closure with a temporary directory as the current directory.
/// This is useful for cleaning up after tests that test code that creates files.
///
/// A mutex is required because `set_current_env` is not thread safe, which
/// causes flaky tests when run in parallel and it's called in multiple tests.
///
/// # Panics
///
/// Panics if the current directory cannot be set to the temporary directory.
pub async fn with_temp_directory<F>(closure: impl FnOnce(&TempDir) -> F)
where
    F: Future<Output = ()>,
{
    // ignore poisoning
    let _guard = ASYNC_CURRENT_DIR_LOCK.lock().await;

    let temp_dir = TempDir::new().unwrap();
    env::set_current_dir(&temp_dir).unwrap();
    closure(&temp_dir).await;
    temp_dir.close().unwrap();
}
