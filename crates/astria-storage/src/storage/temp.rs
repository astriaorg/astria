use std::ops::Deref;

use tempfile::TempDir;

use crate::Storage;

/// A [`Storage`] instance backed by a [`tempfile::TempDir`] for testing.
///
/// The `TempDir` handle is bundled into the `TempStorage`, so the temporary
/// directory is cleaned up when the `TempStorage` instance is dropped.
#[allow(clippy::module_name_repetitions)]
pub struct TempStorage {
    inner: Storage,
    _dir: TempDir,
}

impl Deref for TempStorage {
    type Target = Storage;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TempStorage {
    /// Create a new temporary storage.
    ///
    /// # Errors
    /// Returns an error is the temporary directory holding the storage could
    /// not created, or if a storage could not be created inside that temp dir.
    pub async fn new() -> anyhow::Result<Self> {
        let dir = tempfile::tempdir()?;
        let db_filepath = dir.path().join("storage.db");
        let inner = Storage::load(db_filepath.clone()).await?;

        Ok(TempStorage {
            inner,
            _dir: dir,
        })
    }
}
