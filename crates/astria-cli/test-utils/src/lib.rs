use std::{
    env,
    future::Future,
    sync::{
        Mutex,
        PoisonError,
    },
};

use once_cell::sync::Lazy;
use tempfile::TempDir;
use tokio::sync::Mutex as AsyncMutex;

static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static ASYNC_CURRENT_DIR_LOCK: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

/// Run a closure with a temporary directory as the current directory.
/// This is useful for cleaning up after tests that test code that creates files.
///
/// A mutex is required because `set_current_env` is not thread safe, which
/// causes flaky tests when run in parallel and it's called in multiple tests.
///
/// # Panics
///
/// Panics if the current directory cannot be set to the temporary directory.
pub fn with_temp_directory<F>(closure: F)
where
    F: FnOnce(&TempDir),
{
    // ignore poisoning
    let _guard = CURRENT_DIR_LOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    let temp_dir = TempDir::new().unwrap();
    env::set_current_dir(&temp_dir).unwrap();
    closure(&temp_dir);
    temp_dir.close().unwrap();
}


/// Run an async closure with a temporary directory as the current directory.
/// This is useful for cleaning up after tests that test code that creates files.
///
/// A mutex is required because `set_current_env` is not thread safe, which
/// causes flaky tests when run in parallel and it's called in multiple tests.
///
/// # Panics
///
/// Panics if the current directory cannot be set to the temporary directory.
pub async fn with_temp_directory_async<F>(closure: impl FnOnce(&TempDir) -> F)
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
