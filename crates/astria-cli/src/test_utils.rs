use std::{
    env,
    sync::Mutex,
};

use once_cell::sync::Lazy;
use tempfile::TempDir;

static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Run a closure with a temporary directory as the current directory.
/// This is useful for tests that want to change the current directory.
/// `set_current_env` is not thread safe, so it will cause flaky tests if not behind a mutex.
pub(crate) fn with_temp_directory<F>(closure: F)
where
    F: FnOnce(&TempDir),
{
    // Lock the mutex
    let _guard = CURRENT_DIR_LOCK.lock().unwrap();

    // Store the original current directory
    let original_dir = env::current_dir().unwrap();

    // Create a new temporary directory
    let temp_dir = TempDir::new().unwrap();

    // Change to the temporary directory
    env::set_current_dir(&temp_dir).unwrap();

    // Run the closure, passing it a reference to the temp directory
    closure(&temp_dir);

    // Restore the original current directory
    env::set_current_dir(original_dir).unwrap();

    temp_dir.close().unwrap();
}
