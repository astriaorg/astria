mod benchmarks;

use std::{
    cmp::{
        self,
        Ordering,
    },
    collections::{
        HashMap,
        VecDeque,
    },
    future::Future,
    num::NonZeroUsize,
    sync::{
        Arc,
        OnceLock,
    },
};

use anyhow::Context;
use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
    protocol::transaction::v1alpha1::{
        SignedTransaction,
        TransactionParams,
        UnsignedTransaction,
    },
};
use priority_queue::PriorityQueue;
use tokio::{
    sync::RwLock,
    time::{
        Duration,
        Instant,
    },
};
use tracing::{
    debug,
    instrument,
};

type MempoolQueue = PriorityQueue<EnqueuedTransaction, TransactionPriority>;

/// Used to prioritize transactions in the mempool.
///
/// The priority is calculated as the difference between the transaction nonce and the current
/// account nonce. The lower the difference, the higher the priority.
#[derive(Clone, Debug)]
pub(crate) struct TransactionPriority {
    nonce_diff: u32,
    time_first_seen: Instant,
}

impl PartialEq for TransactionPriority {
    fn eq(&self, other: &Self) -> bool {
        self.nonce_diff == other.nonce_diff
    }
}

impl Eq for TransactionPriority {}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // we want to execute the lowest nonce first,
        // so lower nonce difference means higher priority
        self.nonce_diff.cmp(&other.nonce_diff).reverse()
    }
}

impl PartialOrd for TransactionPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EnqueuedTransaction {
    tx_hash: [u8; 32],
    signed_tx: Arc<SignedTransaction>,
    address: Address,
}

impl EnqueuedTransaction {
    fn new(signed_tx: Arc<SignedTransaction>) -> Self {
        let address = crate::address::base_prefixed(signed_tx.verification_key().address_bytes());
        Self {
            tx_hash: signed_tx.sha256_of_proto_encoding(),
            signed_tx,
            address,
        }
    }

    fn priority(
        &self,
        current_account_nonce: u32,
        time_first_seen: Option<Instant>,
    ) -> anyhow::Result<TransactionPriority> {
        let Some(nonce_diff) = self.signed_tx.nonce().checked_sub(current_account_nonce) else {
            return Err(anyhow::anyhow!(
                "transaction nonce {} is less than current account nonce {current_account_nonce}",
                self.signed_tx.nonce()
            ));
        };

        Ok(TransactionPriority {
            nonce_diff,
            time_first_seen: time_first_seen.unwrap_or(Instant::now()),
        })
    }

    pub(crate) fn tx_hash(&self) -> [u8; 32] {
        self.tx_hash
    }

    pub(crate) fn signed_tx(&self) -> Arc<SignedTransaction> {
        self.signed_tx.clone()
    }

    pub(crate) fn address(&self) -> &Address {
        &self.address
    }
}

/// Only consider `self.tx_hash` for equality. This is consistent with the impl for std `Hash`.
impl PartialEq for EnqueuedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash
    }
}

impl Eq for EnqueuedTransaction {}

/// Only consider `self.tx_hash` when hashing. This is consistent with the impl for equality.
impl std::hash::Hash for EnqueuedTransaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tx_hash.hash(state);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    FailedPrepareProposal(String),
}

const TX_TTL: Duration = Duration::from_secs(600); // 10 minutes
const REMOVAL_CACHE_SIZE: usize = 4096;

/// `RemovalCache` is used to signal to `CometBFT` that a
/// transaction can be removed from the `CometBFT` mempool.
///
/// This is useful for when a transaction fails execution or when a transaction
/// has expired in the app's mempool.
#[derive(Clone)]
pub(crate) struct RemovalCache {
    cache: HashMap<[u8; 32], RemovalReason>,
    remove_queue: VecDeque<[u8; 32]>,
    max_size: NonZeroUsize,
}

impl RemovalCache {
    fn new(max_size: NonZeroUsize) -> Self {
        Self {
            cache: HashMap::new(),
            remove_queue: VecDeque::with_capacity(max_size.into()),
            max_size,
        }
    }

    /// returns Some(RemovalReason) if transaction is cached and
    /// removes the entry from the cache at the same time
    fn remove(&mut self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.cache.remove(&tx_hash)
    }

    /// adds the transaction to the cache
    fn add(&mut self, tx_hash: [u8; 32], reason: RemovalReason) {
        if self.cache.contains_key(&tx_hash) {
            return;
        };

        if self.remove_queue.len() == usize::from(self.max_size) {
            // make space for the new transaction by removing the oldest transaction
            let removed_tx = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            // remove transaction from cache if it is present
            self.cache.remove(&removed_tx);
        }
        self.remove_queue.push_back(tx_hash);
        self.cache.insert(tx_hash, reason);
    }
}

/// [`Mempool`] is an internally-synchronized wrapper around a prioritized queue of transactions
/// awaiting execution.
///
/// The priority is calculated as the difference between the transaction nonce and the current
/// account nonce. The lower the difference, the higher the priority.
///
/// Future extensions to this mempool can include:
/// - maximum mempool size
/// - fee-based ordering
/// - transaction expiration
#[derive(Clone)]
pub(crate) struct Mempool {
    queue: Arc<RwLock<MempoolQueue>>,
    comet_bft_removal_cache: Arc<RwLock<RemovalCache>>,
    tx_ttl: Duration,
}

impl Mempool {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(MempoolQueue::new())),
            comet_bft_removal_cache: Arc::new(RwLock::new(RemovalCache::new(
                NonZeroUsize::try_from(REMOVAL_CACHE_SIZE)
                    .expect("Removal cache cannot be zero sized"),
            ))),
            tx_ttl: TX_TTL,
        }
    }

    /// returns the number of transactions in the mempool
    #[must_use]
    #[instrument(skip_all)]
    pub(crate) async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    /// inserts a transaction into the mempool
    ///
    /// note: the oldest timestamp from found priorities is maintained.
    #[instrument(skip_all)]
    pub(crate) async fn insert(
        &self,
        tx: Arc<SignedTransaction>,
        current_account_nonce: u32,
    ) -> anyhow::Result<()> {
        let enqueued_tx = EnqueuedTransaction::new(tx);
        let fresh_priority = enqueued_tx.priority(current_account_nonce, None)?;
        Self::update_or_insert(&mut *self.queue.write().await, enqueued_tx, &fresh_priority);

        Ok(())
    }

    /// inserts all the given transactions into the mempool
    ///
    /// note: the oldest timestamp from found priorities for an `EnqueuedTransaction` is maintained.
    #[instrument(skip_all)]
    pub(crate) async fn insert_all(&self, txs: Vec<(EnqueuedTransaction, TransactionPriority)>) {
        let mut queue = self.queue.write().await;

        for (enqueued_tx, priority) in txs {
            Self::update_or_insert(&mut queue, enqueued_tx, &priority);
        }
    }

    /// inserts or updates the transaction in a timestamp preserving manner
    ///
    /// note: updates the priority using the `possible_priority`'s nonce diff.
    fn update_or_insert(
        queue: &mut PriorityQueue<EnqueuedTransaction, TransactionPriority>,
        enqueued_tx: EnqueuedTransaction,
        possible_priority: &TransactionPriority,
    ) {
        let oldest_timestamp = queue.get_priority(&enqueued_tx).map_or(
            possible_priority.time_first_seen,
            |prev_priority| {
                possible_priority
                    .time_first_seen
                    .min(prev_priority.time_first_seen)
            },
        );

        let priority = TransactionPriority {
            nonce_diff: possible_priority.nonce_diff,
            time_first_seen: oldest_timestamp,
        };

        let tx_hash = enqueued_tx.tx_hash;
        if queue.push(enqueued_tx, priority).is_none() {
            // emit if didn't already exist
            tracing::trace!(
                tx_hash = %telemetry::display::hex(&tx_hash),
                "inserted transaction into mempool"
            );
        }
    }

    /// pops the transaction with the highest priority from the mempool
    #[must_use]
    #[instrument(skip_all)]
    pub(crate) async fn pop(&self) -> Option<(EnqueuedTransaction, TransactionPriority)> {
        self.queue.write().await.pop()
    }

    /// removes a transaction from the mempool
    #[instrument(skip_all)]
    pub(crate) async fn remove(&self, tx_hash: [u8; 32]) {
        let (signed_tx, address) = dummy_signed_tx();
        let enqueued_tx = EnqueuedTransaction {
            tx_hash,
            signed_tx,
            address,
        };
        self.queue.write().await.remove(&enqueued_tx);
    }

    /// signal that the transaction should be removed from the `CometBFT` mempool
    #[instrument(skip_all)]
    pub(crate) async fn track_removal_comet_bft(&self, tx_hash: [u8; 32], reason: RemovalReason) {
        self.comet_bft_removal_cache
            .write()
            .await
            .add(tx_hash, reason);
    }

    /// checks if a transaction was flagged to be removed from the `CometBFT` mempool
    /// and removes entry
    #[instrument(skip_all)]
    pub(crate) async fn check_removed_comet_bft(&self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.comet_bft_removal_cache.write().await.remove(tx_hash)
    }

    /// Updates the priority of the txs in the mempool based on the current state, and removes any
    /// that are now invalid.
    ///
    /// *NOTE*: this function locks the mempool until every tx has been checked. This could
    /// potentially stall consensus from moving to the next round if the mempool is large.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Address) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        let mut txs_to_remove = Vec::new();
        let mut current_account_nonces = HashMap::new();

        let mut queue = self.queue.write().await;
        let mut removal_cache = self.comet_bft_removal_cache.write().await;
        for (enqueued_tx, priority) in queue.iter_mut() {
            let address = enqueued_tx.address();

            // check if the transactions has expired
            if priority.time_first_seen.elapsed() > self.tx_ttl {
                // tx has expired, set to remove and add to removal cache
                txs_to_remove.push(enqueued_tx.clone());
                removal_cache.add(enqueued_tx.tx_hash, RemovalReason::Expired);
                continue;
            }

            // Try to get the current account nonce from the ones already retrieved.
            let current_account_nonce = if let Some(nonce) = current_account_nonces.get(&address) {
                *nonce
            } else {
                // Fall back to getting via the getter and adding it to the local temp collection.
                let nonce = current_account_nonce_getter(*enqueued_tx.address())
                    .await
                    .context("failed to fetch account nonce")?;
                current_account_nonces.insert(address, nonce);
                nonce
            };
            match enqueued_tx.priority(current_account_nonce, Some(priority.time_first_seen)) {
                Ok(new_priority) => *priority = new_priority,
                Err(e) => {
                    debug!(
                        transaction_hash = %telemetry::display::base64(&enqueued_tx.tx_hash),
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                         "account nonce is now greater than tx nonce; dropping tx from mempool",
                    );
                    txs_to_remove.push(enqueued_tx.clone());
                }
            };
        }

        for enqueued_tx in txs_to_remove {
            queue.remove(&enqueued_tx);
        }

        Ok(())
    }

    /// returns the pending nonce for the given address,
    /// if it exists in the mempool.
    #[instrument(skip_all)]
    pub(crate) async fn pending_nonce(&self, address: &Address) -> Option<u32> {
        let inner = self.queue.read().await;
        let mut nonce = None;
        for (tx, _priority) in inner.iter() {
            if tx.address() == address {
                nonce = Some(cmp::max(nonce.unwrap_or_default(), tx.signed_tx.nonce()));
            }
        }
        nonce
    }
}

/// This exists to provide a `SignedTransaction` for the purposes of removing an entry from the
/// queue where we only have the tx hash available.
///
/// The queue is indexed by `EnqueuedTransaction` which internally needs a `SignedTransaction`, but
/// this `signed_tx` field is ignored in the `PartialEq` and `Hash` impls of `EnqueuedTransaction` -
/// only the tx hash is considered.  So we create an `EnqueuedTransaction` on the fly with the
/// correct tx hash and this dummy signed tx when removing from the queue.
fn dummy_signed_tx() -> (Arc<SignedTransaction>, Address) {
    static TX: OnceLock<(Arc<SignedTransaction>, Address)> = OnceLock::new();
    let (signed_tx, address) = TX.get_or_init(|| {
        let actions = vec![];
        let params = TransactionParams::builder()
            .nonce(0)
            .chain_id("dummy")
            .build();
        let signing_key = SigningKey::from([0; 32]);
        let address = crate::address::base_prefixed(signing_key.verification_key().address_bytes());
        let unsigned_tx = UnsignedTransaction {
            actions,
            params,
        };
        (Arc::new(unsigned_tx.into_signed(&signing_key)), address)
    });
    (signed_tx.clone(), *address)
}

#[cfg(test)]
mod test {
    use std::{
        hash::{
            Hash,
            Hasher,
        },
        time::Duration,
    };

    use super::*;
    use crate::app::test_utils::get_mock_tx;

    #[test]
    fn transaction_priority_should_error_if_invalid() {
        let enqueued_tx = EnqueuedTransaction::new(get_mock_tx(0));
        let priority = enqueued_tx.priority(1, None);
        assert!(
            priority
                .unwrap_err()
                .to_string()
                .contains("less than current account nonce")
        );
    }

    // From https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
    #[test]
    // allow: we want explicit assertions here to match the documented expected behavior.
    #[allow(clippy::nonminimal_bool)]
    fn transaction_priority_comparisons_should_be_consistent() {
        let high = TransactionPriority {
            nonce_diff: 0,
            time_first_seen: Instant::now(),
        };
        let low = TransactionPriority {
            nonce_diff: 1,
            time_first_seen: Instant::now(),
        };

        assert!(high.partial_cmp(&high) == Some(Ordering::Equal));
        assert!(high.partial_cmp(&low) == Some(Ordering::Greater));
        assert!(low.partial_cmp(&high) == Some(Ordering::Less));

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal)
        assert!(high == high); // Some(Equal)
        assert!(!(high == low)); // Some(Greater)
        assert!(!(low == high)); // Some(Less)

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)
        assert!(low < high); // Some(Less)
        assert!(!(high < high)); // Some(Equal)
        assert!(!(high < low)); // Some(Greater)

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert!(high > low); // Some(Greater)
        assert!(!(high > high)); // Some(Equal)
        assert!(!(low > high)); // Some(Less)

        // 4. a <= b if and only if a < b || a == b
        assert!(low <= high); // a < b
        assert!(high <= high); // a == b
        assert!(!(high <= low)); // a > b

        // 5. a >= b if and only if a > b || a == b
        assert!(high >= low); // a > b
        assert!(high >= high); // a == b
        assert!(!(low >= high)); // a < b

        // 6. a != b if and only if !(a == b)
        assert!(high != low); // asserted !(high == low) above
        assert!(low != high); // asserted !(low == high) above
        assert!(!(high != high)); // asserted high == high above
    }

    #[test]
    // From https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq
    fn enqueued_tx_hash_and_eq_should_be_consistent() {
        // Check enqueued txs compare equal if and only if their tx hashes are equal.
        let tx0 = EnqueuedTransaction {
            tx_hash: [0; 32],
            signed_tx: get_mock_tx(0),
            address: crate::address::base_prefixed(
                get_mock_tx(0).verification_key().address_bytes(),
            ),
        };
        let other_tx0 = EnqueuedTransaction {
            tx_hash: [0; 32],
            signed_tx: get_mock_tx(1),
            address: crate::address::base_prefixed(
                get_mock_tx(1).verification_key().address_bytes(),
            ),
        };
        let tx1 = EnqueuedTransaction {
            tx_hash: [1; 32],
            signed_tx: get_mock_tx(0),
            address: crate::address::base_prefixed(
                get_mock_tx(0).verification_key().address_bytes(),
            ),
        };
        assert!(tx0 == other_tx0);
        assert!(tx0 != tx1);

        // Check enqueued txs' std hashes compare equal if and only if their tx hashes are equal.
        let std_hash = |enqueued_tx: &EnqueuedTransaction| -> u64 {
            let mut hasher = std::hash::DefaultHasher::new();
            enqueued_tx.hash(&mut hasher);
            hasher.finish()
        };
        assert!(std_hash(&tx0) == std_hash(&other_tx0));
        assert!(std_hash(&tx0) != std_hash(&tx1));
    }

    #[tokio::test]
    async fn should_insert_and_pop() {
        let mempool = Mempool::new();

        // Priority 0 (highest priority).
        let tx0 = get_mock_tx(0);
        mempool.insert(tx0.clone(), 0).await.unwrap();

        // Priority 1.
        let tx1 = get_mock_tx(1);
        mempool.insert(tx1.clone(), 0).await.unwrap();

        assert_eq!(mempool.len().await, 2);

        // Should pop priority 0 first.
        let (tx, priority) = mempool.pop().await.unwrap();
        assert_eq!(
            tx.signed_tx.sha256_of_proto_encoding(),
            tx0.sha256_of_proto_encoding()
        );
        assert_eq!(priority.nonce_diff, 0);
        assert_eq!(mempool.len().await, 1);

        // Should pop priority 1 second.
        let (tx, priority) = mempool.pop().await.unwrap();
        assert_eq!(
            tx.signed_tx.sha256_of_proto_encoding(),
            tx1.sha256_of_proto_encoding()
        );
        assert_eq!(priority.nonce_diff, 1);
        assert_eq!(mempool.len().await, 0);
    }

    #[tokio::test]
    async fn should_remove() {
        let mempool = Mempool::new();
        let tx_count = 5_usize;

        let current_account_nonce = 0;
        let txs: Vec<_> = (0..tx_count)
            .map(|index| {
                let enqueued_tx =
                    EnqueuedTransaction::new(get_mock_tx(u32::try_from(index).unwrap()));
                let priority = enqueued_tx.priority(current_account_nonce, None).unwrap();
                (enqueued_tx, priority)
            })
            .collect();
        mempool.insert_all(txs.clone()).await;
        assert_eq!(mempool.len().await, tx_count);

        // Remove the last tx.
        let last_tx_hash = txs.last().unwrap().0.tx_hash;
        mempool.remove(last_tx_hash).await;
        let mut expected_remaining_count = tx_count.checked_sub(1).unwrap();
        assert_eq!(mempool.len().await, expected_remaining_count);

        // Removing it again should have no effect.
        mempool.remove(last_tx_hash).await;
        assert_eq!(mempool.len().await, expected_remaining_count);

        // Remove the first tx.
        mempool.remove(txs.first().unwrap().0.tx_hash).await;
        expected_remaining_count = expected_remaining_count.checked_sub(1).unwrap();
        assert_eq!(mempool.len().await, expected_remaining_count);

        // Check the next tx popped is the second priority.
        let (tx, priority) = mempool.pop().await.unwrap();
        assert_eq!(tx.tx_hash, txs[1].0.tx_hash());
        assert_eq!(priority.nonce_diff, 1);
    }

    #[tokio::test]
    async fn should_update_priorities() {
        let mempool = Mempool::new();

        // Insert txs signed by alice with nonces 0 and 1.
        mempool.insert(get_mock_tx(0), 0).await.unwrap();
        mempool.insert(get_mock_tx(1), 0).await.unwrap();

        // Insert txs from a different signer with nonces 100 and 102.
        let other_signing_key = SigningKey::from([1; 32]);
        let other_mock_tx = |nonce: u32| -> Arc<SignedTransaction> {
            let actions = get_mock_tx(0).actions().to_vec();
            let tx = UnsignedTransaction {
                params: TransactionParams::builder()
                    .nonce(nonce)
                    .chain_id("test")
                    .build(),
                actions,
            }
            .into_signed(&other_signing_key);
            Arc::new(tx)
        };
        mempool.insert(other_mock_tx(100), 0).await.unwrap();
        mempool.insert(other_mock_tx(102), 0).await.unwrap();

        assert_eq!(mempool.len().await, 4);

        let (alice_signing_key, alice_address) =
            crate::app::test_utils::get_alice_signing_key_and_address();
        let other_address =
            crate::address::base_prefixed(other_signing_key.verification_key().address_bytes());

        // Create a getter fn which will returns 1 for alice's current account nonce, and 101 for
        // the other signer's.
        let current_account_nonce_getter = |address: Address| async move {
            if address == alice_address {
                return Ok(1);
            }
            if address == other_address {
                return Ok(101);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        // Update the priorities.  Alice's first tx (with nonce 0) and other's first (with nonce
        // 100) should both get purged.
        mempool
            .run_maintenance(current_account_nonce_getter)
            .await
            .unwrap();

        assert_eq!(mempool.len().await, 2);

        // Alice's remaining tx should be the highest priority (nonce diff of 1 - 1 == 0).
        let (tx, priority) = mempool.pop().await.unwrap();
        assert_eq!(tx.signed_tx.nonce(), 1);
        assert_eq!(
            *tx.signed_tx.verification_key(),
            alice_signing_key.verification_key()
        );
        assert_eq!(priority.nonce_diff, 0);

        // Other's remaining tx should be the highest priority (nonce diff of 102 - 101 == 1).
        let (tx, priority) = mempool.pop().await.unwrap();
        assert_eq!(tx.signed_tx.nonce(), 102);
        assert_eq!(
            *tx.signed_tx.verification_key(),
            other_signing_key.verification_key()
        );
        assert_eq!(priority.nonce_diff, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn transaction_timestamp_not_overwritten_insert() {
        let mempool = Mempool::new();

        let insert_time = Instant::now();
        let tx = get_mock_tx(0);
        mempool.insert(tx.clone(), 0).await.unwrap();

        // pass time
        tokio::time::advance(Duration::from_secs(60)).await;
        assert_eq!(
            insert_time.elapsed(),
            Duration::from_secs(60),
            "time should have advanced"
        );

        // re-insert
        mempool.insert(tx, 0).await.unwrap();

        // check that the timestamp was not overwritten in insert()
        let (_, tx_priority) = mempool
            .pop()
            .await
            .expect("transaction was added, should exist");
        assert_eq!(
            tx_priority.time_first_seen.duration_since(insert_time),
            Duration::from_secs(0),
            "Tracked time should be the same"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn transaction_timestamp_not_overwritten_insert_all() {
        let mempool = Mempool::new();

        let insert_time = Instant::now();
        let tx = get_mock_tx(0);
        mempool.insert(tx.clone(), 0).await.unwrap();

        // pass time
        tokio::time::advance(Duration::from_secs(60)).await;
        assert_eq!(
            insert_time.elapsed(),
            Duration::from_secs(60),
            "time should have advanced"
        );

        // re-insert with new priority with higher timestamp
        let enqueued_tx = EnqueuedTransaction::new(tx);
        let new_priority = TransactionPriority {
            nonce_diff: 0,
            time_first_seen: Instant::now(),
        };
        mempool.insert_all(vec![(enqueued_tx, new_priority)]).await;

        // check that the timestamp was not overwritten in insert()
        let (_, tx_priority) = mempool
            .pop()
            .await
            .expect("transaction was added, should exist");
        assert_eq!(
            tx_priority.time_first_seen.duration_since(insert_time),
            Duration::from_secs(0),
            "Tracked time should be the same"
        );
    }

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let mempool = Mempool::new();

        // Insert txs signed by alice with nonces 0 and 1.
        mempool.insert(get_mock_tx(0), 0).await.unwrap();
        mempool.insert(get_mock_tx(1), 0).await.unwrap();

        // Insert txs from a different signer with nonces 100 and 101.
        let other_signing_key = SigningKey::from([1; 32]);
        let other_mock_tx = |nonce: u32| -> Arc<SignedTransaction> {
            let actions = get_mock_tx(0).actions().to_vec();
            let tx = UnsignedTransaction {
                params: TransactionParams::builder()
                    .nonce(nonce)
                    .chain_id("test")
                    .build(),
                actions,
            }
            .into_signed(&other_signing_key);
            Arc::new(tx)
        };
        mempool.insert(other_mock_tx(100), 0).await.unwrap();
        mempool.insert(other_mock_tx(101), 0).await.unwrap();

        assert_eq!(mempool.len().await, 4);

        // Check the pending nonce for alice is 1 and for the other signer is 101.
        let alice_address = crate::app::test_utils::get_alice_signing_key_and_address().1;
        assert_eq!(mempool.pending_nonce(&alice_address).await.unwrap(), 1);
        let other_address =
            crate::address::base_prefixed(other_signing_key.verification_key().address_bytes());
        assert_eq!(mempool.pending_nonce(&other_address).await.unwrap(), 101);

        // Check the pending nonce for an address with no enqueued txs is `None`.
        assert!(
            mempool
                .pending_nonce(&crate::address::base_prefixed([1; 20]))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn tx_cache_size() {
        let mut tx_cache = RemovalCache::new(NonZeroUsize::try_from(2).unwrap());

        let tx_0 = [0u8; 32];
        let tx_1 = [1u8; 32];
        let tx_2 = [2u8; 32];

        assert!(
            tx_cache.remove(tx_0).is_none(),
            "no transaction should be cached at first"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        assert!(
            tx_cache.remove(tx_0).is_some(),
            "transaction was added, should be cached"
        );

        assert!(
            tx_cache.remove(tx_0).is_none(),
            "transaction is cleared after reading"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        tx_cache.add(tx_1, RemovalReason::Expired);
        tx_cache.add(tx_2, RemovalReason::Expired);
        assert!(
            tx_cache.remove(tx_1).is_some(),
            "second transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(tx_2).is_some(),
            "third transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(tx_0).is_none(),
            "first transaction should not be cached"
        );
    }

    #[test]
    fn enqueued_transaction_can_be_instantiated() {
        // This just tests that the constructor does not fail.
        let signed_tx = crate::app::test_utils::get_mock_tx(0);
        let _ = EnqueuedTransaction::new(signed_tx);
    }
}
