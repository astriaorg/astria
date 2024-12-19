use std::{
    fmt::{
        self,
        Debug,
        Formatter,
    },
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
};

use astria_eyre::{
    anyhow_to_eyre,
    eyre,
};
use cnidarium::{
    RootHash,
    StagedWriteBatch,
    StateDelta,
};

pub(crate) use self::{
    snapshot::Snapshot,
    stored_value::StoredValue,
};
use crate::Metrics;

pub(crate) mod keys;
mod snapshot;
mod stored_value;

#[derive(Clone)]
pub(crate) struct Storage {
    inner: cnidarium::Storage,
    latest_snapshot: Arc<Mutex<Snapshot>>,
    metrics: &'static Metrics,
    #[cfg(any(test, feature = "benchmark"))]
    _temp_dir: Option<Arc<tempfile::TempDir>>,
}

impl Storage {
    pub(crate) async fn load(
        path: PathBuf,
        prefixes: Vec<String>,
        metrics: &'static Metrics,
    ) -> astria_eyre::Result<Self> {
        let inner = cnidarium::Storage::load(path, prefixes)
            .await
            .map_err(anyhow_to_eyre)?;
        let latest_snapshot = Arc::new(Mutex::new(Snapshot::new(inner.latest_snapshot(), metrics)));
        Ok(Self {
            inner,
            latest_snapshot,
            metrics,
            #[cfg(any(test, feature = "benchmark"))]
            _temp_dir: None,
        })
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) async fn new_temp() -> Self {
        use telemetry::Metrics as _;

        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| {
            panic!("failed to create temp dir when constructing storage instance: {error}")
        });
        let db_path = temp_dir.path().join("storage.db");
        let inner = cnidarium::Storage::init(db_path.clone(), vec![])
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "failed to initialize storage at `{}`: {error:#}",
                    db_path.display()
                )
            });
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let latest_snapshot = Arc::new(Mutex::new(Snapshot::new(inner.latest_snapshot(), metrics)));

        Self {
            inner,
            latest_snapshot,
            metrics,
            _temp_dir: Some(Arc::new(temp_dir)),
        }
    }

    /// Returns the latest version (block height) of the tree recorded by `Storage`.
    ///
    /// If the tree is empty and has not been initialized, returns `u64::MAX`.
    pub(crate) fn latest_version(&self) -> u64 {
        self.inner.latest_version()
    }

    /// Returns a new `Snapshot` on top of the latest version of the tree.
    pub(crate) fn latest_snapshot(&self) -> Snapshot {
        self.latest_snapshot.lock().unwrap().clone()
    }

    /// Returns the `Snapshot` corresponding to the given version.
    pub(crate) fn snapshot(&self, version: u64) -> Option<Snapshot> {
        Some(Snapshot::new(self.inner.snapshot(version)?, self.metrics))
    }

    /// Returns a new `Delta` on top of the latest version of the tree.
    pub(crate) fn new_delta_of_latest_snapshot(&self) -> StateDelta<Snapshot> {
        self.latest_snapshot().new_delta()
    }

    /// Returns a clone of the wrapped `cnidarium::Storage`.
    pub(crate) fn inner(&self) -> cnidarium::Storage {
        self.inner.clone()
    }

    /// Prepares a commit for the provided `SnapshotDelta`, returning a `StagedWriteBatch`.
    ///
    /// The batch can be committed to the database using the [`Storage::commit_batch`] method.
    pub(crate) async fn prepare_commit(
        &self,
        delta: StateDelta<Snapshot>,
    ) -> eyre::Result<StagedWriteBatch> {
        let (snapshot, changes) = delta.flatten();
        let cnidarium_snapshot = snapshot.into_inner();
        let mut cnidarium_delta = StateDelta::new(cnidarium_snapshot);
        changes.apply_to(&mut cnidarium_delta);
        self.inner
            .prepare_commit(cnidarium_delta)
            .await
            .map_err(anyhow_to_eyre)
    }

    /// Commits the provided `SnapshotDelta` to persistent storage as the latest version of the
    /// chain state.
    #[cfg(test)]
    pub(crate) async fn commit(&self, delta: StateDelta<Snapshot>) -> eyre::Result<RootHash> {
        let batch = self.prepare_commit(delta).await?;
        self.commit_batch(batch)
    }

    /// Commits the supplied `StagedWriteBatch` to persistent storage.
    pub(crate) fn commit_batch(&self, batch: StagedWriteBatch) -> eyre::Result<RootHash> {
        let root_hash = self.inner.commit_batch(batch).map_err(anyhow_to_eyre)?;
        let mut ls = self.latest_snapshot.lock().unwrap();
        *ls = Snapshot::new(self.inner.latest_snapshot(), self.metrics);
        Ok(root_hash)
    }

    /// Shuts down the database and the dispatcher task, and waits for all resources to be
    /// reclaimed.
    ///
    /// # Panics
    ///
    /// Panics if there is more than one clone remaining of the `cnidarium::Inner` storage `Arc`.
    pub(crate) async fn release(self) {
        self.inner.release().await;
    }
}

impl Debug for Storage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use cnidarium::{
        StateRead as _,
        StateWrite as _,
    };
    use telemetry::Metrics as _;

    use super::*;

    const V_KEY: &str = "verifiable key";
    const NV_KEY: &[u8] = b"non-verifiable key";
    const VALUES: [[u8; 1]; 4] = [[1], [2], [3], [4]];

    #[test]
    fn should_prepare_and_commit_batch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("storage_test");
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));

        // Run the tests on the first storage instance.
        //
        // NOTE: `cnidarium::Storage::load` panics if we try to open it more than once from the same
        // thread, even if the first instance is dropped. We use two separate tokio runtimes to
        // avoid this.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let storage = Storage::load(db_path.clone(), vec![], metrics)
                    .await
                    .unwrap();

                // Check there's no previous snapshots available.
                assert!(storage.snapshot(0).is_none());

                // Write data to the verifiable and non-verifiable stores.
                let mut state_delta = storage.new_delta_of_latest_snapshot();
                state_delta.put_raw(V_KEY.to_string(), VALUES[0].to_vec());
                state_delta.nonverifiable_put_raw(NV_KEY.to_vec(), VALUES[1].to_vec());

                // Commit the data.
                let batch = storage.prepare_commit(state_delta).await.unwrap();
                storage.commit_batch(batch).unwrap();

                // Check the data is available in a new latest snapshot, and a snapshot at v0
                // (the only version currently available).
                let snapshot_0 = storage.snapshot(0).unwrap();
                assert_eq!(
                    Some(VALUES[0].to_vec()),
                    snapshot_0.get_raw(V_KEY).await.unwrap()
                );
                assert_eq!(
                    Some(VALUES[1].to_vec()),
                    snapshot_0.nonverifiable_get_raw(NV_KEY).await.unwrap()
                );

                let snapshot_latest = storage.latest_snapshot();
                assert_eq!(
                    Some(VALUES[0].to_vec()),
                    snapshot_latest.get_raw(V_KEY).await.unwrap()
                );
                assert_eq!(
                    Some(VALUES[1].to_vec()),
                    snapshot_latest.nonverifiable_get_raw(NV_KEY).await.unwrap()
                );

                // Check there's no snapshot v1.
                assert!(storage.snapshot(1).is_none());

                // Shut down the original storage instance.
                storage.release().await;
            });

        // Open a new storage instance using the same DB file and run follow-up tests.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let storage = Storage::load(db_path.clone(), vec![], metrics)
                    .await
                    .unwrap();

                // Check the data is available in snapshot v0 (the only snapshot available now).
                let snapshot_0 = storage.snapshot(0).unwrap();
                assert_eq!(
                    Some(VALUES[0].to_vec()),
                    snapshot_0.get_raw(V_KEY).await.unwrap()
                );
                assert_eq!(
                    Some(VALUES[1].to_vec()),
                    snapshot_0.nonverifiable_get_raw(NV_KEY).await.unwrap()
                );

                // Overwrite the values and commit these changes.
                let mut state_delta = storage.new_delta_of_latest_snapshot();
                state_delta.put_raw(V_KEY.to_string(), VALUES[2].to_vec());
                state_delta.nonverifiable_put_raw(NV_KEY.to_vec(), VALUES[3].to_vec());
                let batch = storage.prepare_commit(state_delta).await.unwrap();
                storage.commit_batch(batch).unwrap();

                // Check the data has the original values in snapshot v0, but the new values in
                // the latest snapshot (v1).
                let snapshot_0 = storage.snapshot(0).unwrap();
                assert_eq!(
                    Some(VALUES[0].to_vec()),
                    snapshot_0.get_raw(V_KEY).await.unwrap()
                );
                assert_eq!(
                    Some(VALUES[1].to_vec()),
                    snapshot_0.nonverifiable_get_raw(NV_KEY).await.unwrap()
                );

                let snapshot_latest = storage.latest_snapshot();
                assert_eq!(
                    Some(VALUES[2].to_vec()),
                    snapshot_latest.get_raw(V_KEY).await.unwrap()
                );
                assert_eq!(
                    Some(VALUES[3].to_vec()),
                    snapshot_latest.nonverifiable_get_raw(NV_KEY).await.unwrap()
                );

                // Check snapshot v1 exists, and there's no snapshot v2.
                assert!(storage.snapshot(1).is_some());
                assert!(storage.snapshot(2).is_none());
            });
    }

    #[tokio::test]
    async fn should_not_commit_invalid_batch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("storage_test");
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let storage = Storage::load(db_path.clone(), vec![], metrics)
            .await
            .unwrap();

        // Write and commit data twice.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.put_raw(V_KEY.to_string(), VALUES[0].to_vec());
        storage.commit(state_delta).await.unwrap();

        state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.nonverifiable_put_raw(NV_KEY.to_vec(), VALUES[1].to_vec());
        storage.commit(state_delta).await.unwrap();

        // Assert we have two snapshot versions available.
        assert!(storage.snapshot(0).is_some());
        assert!(storage.snapshot(1).is_some());
        assert!(storage.snapshot(2).is_none());

        // Create a new state delta from snapshot v0 and try to commit it - should fail.
        state_delta = storage.snapshot(0).unwrap().new_delta();
        match storage.prepare_commit(state_delta).await {
            Ok(_) => panic!("should fail to prepare commit for an existing snapshot version"),
            Err(error) => {
                assert!(error.to_string().contains(
                    "trying to prepare a commit for a delta forked from version 0, but the latest \
                     version is 1"
                ));
            }
        }

        // Assert we still have two snapshot versions available.
        assert!(storage.snapshot(0).is_some());
        assert!(storage.snapshot(1).is_some());
        assert!(storage.snapshot(2).is_none());
    }
}
