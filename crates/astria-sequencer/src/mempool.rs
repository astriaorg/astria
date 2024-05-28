use std::{
    cmp::{
        self,
        Ordering,
    },
    collections::HashMap,
    future::Future,
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
use tokio::sync::RwLock;
use tracing::debug;

type MempoolQueue = PriorityQueue<EnqueuedTransaction, TransactionPriority>;

/// Used to prioritize transactions in the mempool.
///
/// The priority is calculated as the difference between the transaction nonce and the current
/// account nonce. The lower the difference, the higher the priority.
#[derive(Clone, Debug)]
pub(crate) struct TransactionPriority {
    nonce_diff: u32,
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
}

impl EnqueuedTransaction {
    fn new(signed_tx: SignedTransaction) -> Self {
        Self {
            tx_hash: signed_tx.sha256_of_proto_encoding(),
            signed_tx: Arc::new(signed_tx),
        }
    }

    fn priority(&self, current_account_nonce: u32) -> anyhow::Result<TransactionPriority> {
        let Some(nonce_diff) = self.signed_tx.nonce().checked_sub(current_account_nonce) else {
            return Err(anyhow::anyhow!(
                "transaction nonce {} is less than current account nonce {current_account_nonce}",
                self.signed_tx.nonce()
            ));
        };

        Ok(TransactionPriority {
            nonce_diff,
        })
    }

    pub(crate) fn tx_hash(&self) -> [u8; 32] {
        self.tx_hash
    }

    pub(crate) fn signed_tx(&self) -> Arc<SignedTransaction> {
        self.signed_tx.clone()
    }

    pub(crate) fn address(&self) -> &Address {
        self.signed_tx.verification_key().address()
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
    inner: Arc<RwLock<MempoolQueue>>,
}

impl Mempool {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MempoolQueue::new())),
        }
    }

    /// returns the number of transactions in the mempool
    #[must_use]
    pub(crate) async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// inserts a transaction into the mempool
    ///
    /// note: if the tx already exists in the mempool, it's overwritten with the new priority.
    pub(crate) async fn insert(
        &self,
        tx: SignedTransaction,
        current_account_nonce: u32,
    ) -> anyhow::Result<()> {
        let enqueued_tx = EnqueuedTransaction::new(tx);
        let priority = enqueued_tx.priority(current_account_nonce)?;
        let tx_hash = enqueued_tx.tx_hash;
        self.inner.write().await.push(enqueued_tx, priority);
        tracing::trace!(
            tx_hash = %telemetry::display::hex(&tx_hash),
            "inserted transaction into mempool"
        );
        Ok(())
    }

    /// inserts all the given transactions into the mempool
    pub(crate) async fn insert_all(&self, txs: Vec<(EnqueuedTransaction, TransactionPriority)>) {
        self.inner.write().await.extend(txs);
    }

    /// pops the transaction with the highest priority from the mempool
    #[must_use]
    pub(crate) async fn pop(&self) -> Option<(EnqueuedTransaction, TransactionPriority)> {
        self.inner.write().await.pop()
    }

    /// removes a transaction from the mempool
    pub(crate) async fn remove(&self, tx_hash: [u8; 32]) {
        let enqueued_tx = EnqueuedTransaction {
            tx_hash,
            signed_tx: dummy_signed_tx().clone(),
        };
        self.inner.write().await.remove(&enqueued_tx);
    }

    /// Updates the priority of the txs in the mempool based on the current state, and removes any
    /// that are now invalid.
    ///
    /// *NOTE*: this function locks the mempool until every tx has been checked. This could
    /// potentially stall consensus from moving to the next round if the mempool is large.
    pub(crate) async fn update_priorities<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Address) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        let mut txs_to_remove = Vec::new();
        let mut current_account_nonces = HashMap::new();

        let mut queue = self.inner.write().await;
        for (enqueued_tx, priority) in queue.iter_mut() {
            let address = enqueued_tx.address();
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
            match enqueued_tx.priority(current_account_nonce) {
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
    pub(crate) async fn pending_nonce(&self, address: &Address) -> Option<u32> {
        let inner = self.inner.read().await;
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
fn dummy_signed_tx() -> &'static Arc<SignedTransaction> {
    static TX: OnceLock<Arc<SignedTransaction>> = OnceLock::new();
    TX.get_or_init(|| {
        let actions = vec![];
        let params = TransactionParams {
            nonce: 0,
            chain_id: String::new(),
        };
        let signing_key = SigningKey::from([0; 32]);
        let unsigned_tx = UnsignedTransaction {
            actions,
            params,
        };
        Arc::new(unsigned_tx.into_signed(&signing_key))
    })
}

#[cfg(test)]
mod test {
    use std::hash::{
        Hash,
        Hasher,
    };

    use super::*;
    use crate::app::test_utils::get_mock_tx;

    #[test]
    fn transaction_priority_should_error_if_invalid() {
        let enqueued_tx = EnqueuedTransaction::new(get_mock_tx(0));
        let priority = enqueued_tx.priority(1);
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
        };
        let low = TransactionPriority {
            nonce_diff: 1,
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
            signed_tx: Arc::new(get_mock_tx(0)),
        };
        let other_tx0 = EnqueuedTransaction {
            tx_hash: [0; 32],
            signed_tx: Arc::new(get_mock_tx(1)),
        };
        let tx1 = EnqueuedTransaction {
            tx_hash: [1; 32],
            signed_tx: Arc::new(get_mock_tx(0)),
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
                let priority = enqueued_tx.priority(current_account_nonce).unwrap();
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
        let other_mock_tx = |nonce: u32| -> SignedTransaction {
            let actions = get_mock_tx(0).actions().to_vec();
            UnsignedTransaction {
                params: TransactionParams {
                    nonce,
                    chain_id: "test".to_string(),
                },
                actions,
            }
            .into_signed(&other_signing_key)
        };
        mempool.insert(other_mock_tx(100), 0).await.unwrap();
        mempool.insert(other_mock_tx(102), 0).await.unwrap();

        assert_eq!(mempool.len().await, 4);

        let (alice_signing_key, alice_address) =
            crate::app::test_utils::get_alice_signing_key_and_address();
        let other_address = *other_signing_key.verification_key().address();

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
            .update_priorities(current_account_nonce_getter)
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

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let mempool = Mempool::new();

        // Insert txs signed by alice with nonces 0 and 1.
        mempool.insert(get_mock_tx(0), 0).await.unwrap();
        mempool.insert(get_mock_tx(1), 0).await.unwrap();

        // Insert txs from a different signer with nonces 100 and 101.
        let other_signing_key = SigningKey::from([1; 32]);
        let other_mock_tx = |nonce: u32| -> SignedTransaction {
            let actions = get_mock_tx(0).actions().to_vec();
            UnsignedTransaction {
                params: TransactionParams {
                    nonce,
                    chain_id: "test".to_string(),
                },
                actions,
            }
            .into_signed(&other_signing_key)
        };
        mempool.insert(other_mock_tx(100), 0).await.unwrap();
        mempool.insert(other_mock_tx(101), 0).await.unwrap();

        assert_eq!(mempool.len().await, 4);

        // Check the pending nonce for alice is 1 and for the other signer is 101.
        let alice_address = crate::app::test_utils::get_alice_signing_key_and_address().1;
        assert_eq!(mempool.pending_nonce(&alice_address).await.unwrap(), 1);
        let other_address = *other_signing_key.verification_key().address();
        assert_eq!(mempool.pending_nonce(&other_address).await.unwrap(), 101);

        // Check the pending nonce for an address with no enqueued txs is `None`.
        assert!(
            mempool
                .pending_nonce(&Address::from([1; 20]))
                .await
                .is_none()
        );
    }
}
