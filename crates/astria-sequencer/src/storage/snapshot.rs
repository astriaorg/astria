use std::{
    any::{
        Any,
        TypeId,
    },
    fmt::{
        self,
        Debug,
        Formatter,
    },
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use anyhow::Context as _;
use astria_eyre::anyhow_to_eyre;
use async_trait::async_trait;
use bytes::Bytes;
use cnidarium::{
    RootHash,
    StateDelta,
    StateRead,
};
use futures::TryStreamExt;
use pin_project_lite::pin_project;
use quick_cache::sync::Cache as QuickCache;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::Metrics;

/// An in-memory cache of objects that belong in the verifiable store.
///
/// A `None` value represents an item not present in the on-disk storage.
type VerifiableCache = Arc<QuickCache<String, Option<Bytes>>>;
/// An in-memory cache of objects that belong in the non-verifiable store.
///
/// A `None` value represents an item not present in the on-disk storage.
type NonVerifiableCache = Arc<QuickCache<Vec<u8>, Option<Bytes>>>;

#[derive(Clone)]
pub(crate) struct Snapshot {
    inner: cnidarium::Snapshot,
    verifiable_cache: VerifiableCache,
    non_verifiable_cache: NonVerifiableCache,
    metrics: &'static Metrics,
}

impl Snapshot {
    pub(super) fn new(inner: cnidarium::Snapshot, metrics: &'static Metrics) -> Self {
        Self {
            inner,
            verifiable_cache: Arc::new(QuickCache::new(10_000)),
            non_verifiable_cache: Arc::new(QuickCache::new(1_000)),
            metrics,
        }
    }

    pub(super) fn into_inner(self) -> cnidarium::Snapshot {
        self.inner
    }

    pub(crate) fn new_delta(&self) -> StateDelta<Snapshot> {
        StateDelta::new(self.clone())
    }

    pub(crate) async fn root_hash(&self) -> astria_eyre::Result<RootHash> {
        self.inner.root_hash().await.map_err(anyhow_to_eyre)
    }
}

impl Debug for Snapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[async_trait]
impl StateRead for Snapshot {
    type GetRawFut = SnapshotFuture;
    type NonconsensusPrefixRawStream = ReceiverStream<anyhow::Result<(Vec<u8>, Vec<u8>)>>;
    type NonconsensusRangeRawStream = ReceiverStream<anyhow::Result<(Vec<u8>, Vec<u8>)>>;
    type PrefixKeysStream = ReceiverStream<anyhow::Result<String>>;
    type PrefixRawStream = ReceiverStream<anyhow::Result<(String, Vec<u8>)>>;

    fn get_raw(&self, key: &str) -> Self::GetRawFut {
        get_raw(
            key.to_owned(),
            self.inner.clone(),
            self.verifiable_cache.clone(),
            self.metrics,
        )
    }

    fn nonverifiable_get_raw(&self, key: &[u8]) -> Self::GetRawFut {
        non_verifiable_get_raw(
            key.to_owned(),
            self.inner.clone(),
            self.non_verifiable_cache.clone(),
            self.metrics,
        )
    }

    fn object_get<T: Any + Send + Sync + Clone>(&self, _key: &str) -> Option<T> {
        // No ephemeral object cache in read-only `Snapshot`.
        None
    }

    fn object_type(&self, _key: &str) -> Option<TypeId> {
        // No ephemeral object cache in read-only `Snapshot`.
        None
    }

    fn prefix_raw(&self, prefix: &str) -> Self::PrefixRawStream {
        let (tx_prefix_item, rx_prefix_query) = mpsc::channel(10);
        let inner_snapshot = self.inner.clone();
        let cache = self.verifiable_cache.clone();
        let metrics = self.metrics;
        tokio::spawn(inner_snapshot.prefix_keys(prefix).try_for_each(move |key| {
            let inner_snapshot = inner_snapshot.clone();
            let cache = cache.clone();
            let tx_prefix_item = tx_prefix_item.clone();
            async move {
                let value = get_raw(key.clone(), inner_snapshot, cache, metrics)
                    .await?
                    .with_context(|| "should never be `None` value for streamed key")?;
                let permit = tx_prefix_item
                    .reserve()
                    .await
                    .with_context(|| "failed to reserve space on the sending channel")?;
                permit.send(Ok((key, value)));
                Ok(())
            }
        }));
        ReceiverStream::new(rx_prefix_query)
    }

    fn prefix_keys(&self, prefix: &str) -> Self::PrefixKeysStream {
        self.inner.prefix_keys(prefix)
    }

    /// NOTE: The cache is unusable here.
    fn nonverifiable_prefix_raw(&self, prefix: &[u8]) -> Self::NonconsensusPrefixRawStream {
        self.inner.nonverifiable_prefix_raw(prefix)
    }

    /// NOTE: The cache is unusable here.
    fn nonverifiable_range_raw(
        &self,
        prefix: Option<&[u8]>,
        range: impl std::ops::RangeBounds<Vec<u8>>,
    ) -> anyhow::Result<Self::NonconsensusRangeRawStream> {
        self.inner.nonverifiable_range_raw(prefix, range)
    }
}

pin_project! {
    pub struct SnapshotFuture {
        #[pin]
        inner: tokio::task::JoinHandle<anyhow::Result<Option<Vec<u8>>>>
    }
}

impl SnapshotFuture {
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = anyhow::Result<Option<Vec<u8>>>> + Send + 'static,
    {
        Self {
            inner: tokio::task::spawn(future),
        }
    }
}

impl Future for SnapshotFuture {
    type Output = anyhow::Result<Option<Vec<u8>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(result) => {
                Poll::Ready(result.expect("unrecoverable join error from tokio task"))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn get<S, K>(state: &S, key: K) -> anyhow::Result<Option<Vec<u8>>>
where
    S: StateRead + ?Sized,
    K: AsRef<str>,
{
    let key = key.as_ref();
    state
        .get_raw(key)
        .await
        .with_context(|| format!("failed to get raw value under key `{key}`"))
}

async fn non_verifiable_get<S, K>(state: &S, key: K) -> anyhow::Result<Option<Vec<u8>>>
where
    S: StateRead,
    K: AsRef<[u8]>,
{
    let key = key.as_ref();
    state.nonverifiable_get_raw(key).await.with_context(|| {
        format!(
            "failed to get nonverifiable raw value under key `{}`",
            display_non_verifiable_key(key)
        )
    })
}

fn get_raw(
    key: String,
    inner_snapshot: cnidarium::Snapshot,
    cache: VerifiableCache,
    metrics: &'static Metrics,
) -> SnapshotFuture {
    SnapshotFuture::new(async move {
        let maybe_value = match cache.get_value_or_guard_async(&key).await {
            Ok(value) => {
                metrics.increment_verifiable_cache_hit();
                value
            }
            Err(guard) => {
                metrics.increment_verifiable_cache_miss();
                let value = get(&inner_snapshot, &key).await?.map(Bytes::from);
                let _ = guard.insert(value.clone());
                value
            }
        };
        metrics.record_verifiable_cache_item_total(cache.len());
        Ok(maybe_value.map(Vec::from))
    })
}

fn non_verifiable_get_raw(
    key: Vec<u8>,
    inner_snapshot: cnidarium::Snapshot,
    cache: NonVerifiableCache,
    metrics: &'static Metrics,
) -> SnapshotFuture {
    SnapshotFuture::new(async move {
        let maybe_value = match cache.get_value_or_guard_async(&key).await {
            Ok(value) => {
                metrics.increment_non_verifiable_cache_hit();
                value
            }
            Err(guard) => {
                metrics.increment_non_verifiable_cache_miss();
                let value = non_verifiable_get(&inner_snapshot, &key)
                    .await?
                    .map(Bytes::from);
                let _ = guard.insert(value.clone());
                value
            }
        };
        metrics.record_non_verifiable_cache_item_total(cache.len());
        Ok(maybe_value.map(Vec::from))
    })
}

/// Provides a `String` version of the given key for display (logging) purposes, parsed from UTF-8
/// if possible, falling back to base64 encoding.
fn display_non_verifiable_key(key: &[u8]) -> String {
    String::from_utf8(key.to_vec()).unwrap_or_else(|_| telemetry::display::base64(key).to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cnidarium::StateWrite as _;
    use tempfile::TempDir;

    use super::{
        super::Storage,
        *,
    };

    const V_KEY: &str = "verifiable key";
    const NV_KEY: &[u8] = b"non-verifiable key";
    const VALUES: [[u8; 1]; 4] = [[1], [2], [3], [4]];

    struct Fixture {
        storage: Storage,
        _temp_dir: TempDir,
    }

    impl Fixture {
        async fn new() -> Self {
            let (metrics, _) = telemetry::metrics::ConfigBuilder::new()
                .set_global_recorder(false)
                .build(&())
                .unwrap();
            let metrics = Box::leak(Box::new(metrics));
            let temp_dir = tempfile::tempdir().unwrap();
            let db_path = temp_dir.path().join("storage_test");
            let storage = Storage::load(db_path.clone(), vec![], metrics)
                .await
                .unwrap();
            Self {
                storage,
                _temp_dir: temp_dir,
            }
        }
    }

    #[tokio::test]
    async fn get_raw_should_succeed() {
        #[track_caller]
        fn assert_in_cache(snapshot: &Snapshot, value: &[u8]) {
            let Some(serialized_value) = snapshot.verifiable_cache.get(V_KEY).unwrap() else {
                panic!("should have value in cache");
            };
            assert_eq!(value.to_vec(), serialized_value);
        }

        let Fixture {
            storage,
            _temp_dir,
        } = Fixture::new().await;

        // `get_raw` should return `None` for non-existent value.
        let snapshot = storage.latest_snapshot();
        assert!(snapshot.get_raw(V_KEY).await.unwrap().is_none());

        // Write and commit data.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.put_raw(V_KEY.to_string(), VALUES[0].to_vec());
        storage.commit(state_delta).await.unwrap();

        // `get_raw` on the latest snapshot should return the correct value and cache it.
        let snapshot = storage.latest_snapshot();
        assert!(snapshot.verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[0].to_vec()),
            snapshot.get_raw(V_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[0]);

        // Write and commit different data under the same key.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.put_raw(V_KEY.to_string(), VALUES[1].to_vec());
        storage.commit(state_delta).await.unwrap();

        // `get_raw` on a v0 snapshot should return the original value, and on the latest snapshot
        // should return the updated value. Both caches should be updated.
        let snapshot = storage.snapshot(0).unwrap();
        assert!(snapshot.verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[0].to_vec()),
            snapshot.get_raw(V_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[0]);

        let snapshot = storage.latest_snapshot();
        assert!(snapshot.verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[1].to_vec()),
            snapshot.get_raw(V_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[1]);

        // Check a clone of the latest snapshot has clone of the populated cache.
        assert_eq!(1, storage.latest_snapshot().verifiable_cache.len());
        assert_in_cache(&storage.latest_snapshot(), &VALUES[1]);
    }

    #[tokio::test]
    async fn nonverifiable_get_raw_should_succeed() {
        #[track_caller]
        fn assert_in_cache(snapshot: &Snapshot, value: &[u8]) {
            let Some(serialized_value) = snapshot.non_verifiable_cache.get(NV_KEY).unwrap() else {
                panic!("should have value in cache");
            };
            assert_eq!(value.to_vec(), serialized_value);
        }

        let Fixture {
            storage,
            _temp_dir,
        } = Fixture::new().await;

        // `nonverifiable_get_raw` should return `None` for non-existent value.
        let snapshot = storage.latest_snapshot();
        assert!(
            snapshot
                .nonverifiable_get_raw(NV_KEY)
                .await
                .unwrap()
                .is_none()
        );

        // Write and commit data.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.nonverifiable_put_raw(NV_KEY.to_vec(), VALUES[0].to_vec());
        storage.commit(state_delta).await.unwrap();

        // `nonverifiable_get_raw` on the latest snapshot should return the correct value and cache
        // it.
        let snapshot = storage.latest_snapshot();
        assert!(snapshot.non_verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[0].to_vec()),
            snapshot.nonverifiable_get_raw(NV_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.non_verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[0]);

        // Write and commit different data under the same key.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        state_delta.nonverifiable_put_raw(NV_KEY.to_vec(), VALUES[1].to_vec());
        storage.commit(state_delta).await.unwrap();

        // `nonverifiable_get_raw` on a v0 snapshot should return the original value, and on the
        // latest snapshot should return the updated value. Both caches should be updated.
        let snapshot = storage.snapshot(0).unwrap();
        assert!(snapshot.non_verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[0].to_vec()),
            snapshot.nonverifiable_get_raw(NV_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.non_verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[0]);

        let snapshot = storage.latest_snapshot();
        assert!(snapshot.non_verifiable_cache.is_empty());
        assert_eq!(
            Some(VALUES[1].to_vec()),
            snapshot.nonverifiable_get_raw(NV_KEY).await.unwrap()
        );
        assert_eq!(1, snapshot.non_verifiable_cache.len());
        assert_in_cache(&snapshot, &VALUES[1]);

        // Check a clone of the latest snapshot has clone of the populated cache.
        assert_eq!(1, storage.latest_snapshot().non_verifiable_cache.len());
        assert_in_cache(&storage.latest_snapshot(), &VALUES[1]);
    }

    #[tokio::test]
    async fn prefix_raw_should_succeed() {
        let Fixture {
            storage,
            _temp_dir,
        } = Fixture::new().await;

        // `prefix_raw` should return an empty stream for a non-existent prefix.
        let snapshot = storage.latest_snapshot();
        let map: BTreeMap<_, _> = snapshot.prefix_raw(V_KEY).try_collect().await.unwrap();
        assert!(map.is_empty());

        // Write and commit four entries under a common prefix.
        let mut state_delta = storage.new_delta_of_latest_snapshot();
        let kv_iter = VALUES
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("common {index}"), value.to_vec()));
        for (key, value) in kv_iter.clone() {
            state_delta.put_raw(key, value);
        }
        storage.commit(state_delta).await.unwrap();

        // Get a new snapshot, and populate its inner cache with two of the stored values by getting
        // them.
        let snapshot = storage.latest_snapshot();
        assert!(snapshot.verifiable_cache.is_empty());
        assert!(snapshot.get_raw("common 0").await.unwrap().is_some());
        assert!(snapshot.get_raw("common 2").await.unwrap().is_some());
        assert_eq!(2, snapshot.verifiable_cache.len());

        // `prefix_raw` should return all the key value pairs and populate the cache.
        let actual: BTreeMap<_, _> = snapshot.prefix_raw("com").try_collect().await.unwrap();
        let expected: BTreeMap<_, _> = kv_iter.collect();
        assert_eq!(expected, actual);
        assert_eq!(4, snapshot.verifiable_cache.len());
    }
}
