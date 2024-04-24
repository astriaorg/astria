use std::{
    cmp::Ordering,
    collections::HashMap,
};

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use priority_queue::double_priority_queue::DoublePriorityQueue;

/// Used to prioritize transactions in the mempool.
///
/// The priority is calculated as the difference between the transaction nonce and the current
/// account nonce. The lower the difference, the higher the priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransactionPriority {
    transaction_nonce: u32,
    current_account_nonce: u32,
}

impl PartialOrd for TransactionPriority {
    #[allow(clippy::non_canonical_partial_ord_impl)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_nonce_diff = self.transaction_nonce - self.current_account_nonce;
        let other_nonce_diff = other.transaction_nonce - other.current_account_nonce;

        // we want to execute the lowest nonce first,
        // so lower nonce difference means higher priority
        #[allow(clippy::comparison_chain)]
        if self_nonce_diff > other_nonce_diff {
            Some(Ordering::Less)
        } else if self_nonce_diff < other_nonce_diff {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl TransactionPriority {
    pub(crate) fn new(transaction_nonce: u32, current_account_nonce: u32) -> anyhow::Result<Self> {
        if transaction_nonce < current_account_nonce {
            return Err(anyhow::anyhow!(
                "transaction nonce {} is less than current account nonce {}",
                transaction_nonce,
                current_account_nonce
            ));
        }

        Ok(Self {
            transaction_nonce,
            current_account_nonce,
        })
    }
}

/// [`BasicMempool`] is a simple mempool implementation that stores transactions in a priority queue
/// ordered by nonce.
///
/// Future extensions to this mempool can include:
/// - maximum mempool size
/// - fee-based ordering
/// - transaction expiration
pub(crate) struct BasicMempool {
    queue: DoublePriorityQueue<[u8; 32], TransactionPriority>,
    hash_to_tx: HashMap<[u8; 32], SignedTransaction>,
}

impl BasicMempool {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            queue: DoublePriorityQueue::new(),
            hash_to_tx: HashMap::new(),
        }
    }

    /// returns the number of transactions in the mempool
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }

    /// inserts a transaction into the mempool
    ///
    /// note: if the tx already exists in the mempool, it's overwritten with the new priority.
    pub(crate) fn insert(
        &mut self,
        tx: SignedTransaction,
        priority: TransactionPriority,
    ) -> anyhow::Result<()> {
        if tx.nonce() != priority.transaction_nonce {
            anyhow::bail!("transaction nonce does not match `transaction_nonce` in priority");
        }

        let hash = tx.sha256_of_proto_encoding();
        self.queue.push(hash, priority);
        self.hash_to_tx.insert(hash, tx);
        Ok(())
    }

    /// inserts all the given transactions into the mempool
    pub(crate) fn insert_all(
        &mut self,
        txs: Vec<(SignedTransaction, TransactionPriority)>,
    ) -> anyhow::Result<()> {
        for (tx, priority) in txs {
            self.insert(tx, priority)?;
        }
        Ok(())
    }

    /// pops the transaction with the highest priority from the mempool
    #[must_use]
    pub(crate) fn pop(&mut self) -> Option<(SignedTransaction, TransactionPriority)> {
        let (hash, priority) = self.queue.pop_max()?;
        let tx = self.hash_to_tx.remove(&hash)?;
        Some((tx, priority))
    }

    /// removes a transaction from the mempool
    pub(crate) fn remove(&mut self, tx_hash: &[u8; 32]) {
        self.queue.remove(tx_hash);
        self.hash_to_tx.remove(tx_hash);
    }

    /// removes all the given transactions from the mempool
    pub(crate) fn remove_all(&mut self, tx_hashes: &Vec<[u8; 32]>) {
        for tx_hash in tx_hashes {
            self.remove(tx_hash);
        }
    }

    #[must_use]
    pub(crate) fn iter_mut(&mut self) -> BasicMempoolIterMut {
        BasicMempoolIterMut {
            iter: self.queue.iter_mut(),
            hash_to_tx: &mut self.hash_to_tx,
        }
    }
}

pub(crate) struct BasicMempoolIterMut<'a> {
    iter: priority_queue::double_priority_queue::iterators::IterMut<
        'a,
        [u8; 32],
        TransactionPriority,
    >,
    hash_to_tx: &'a HashMap<[u8; 32], SignedTransaction>,
}

impl<'a> Iterator for BasicMempoolIterMut<'a> {
    type Item = (&'a SignedTransaction, &'a mut TransactionPriority);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(hash, priority)| {
            let tx = self
                .hash_to_tx
                .get(hash)
                .expect("hash in queue must be in hash_to_tx");
            (tx, priority)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::test_utils::get_mock_tx;

    #[test]
    fn mempool_nonce_priority() {
        let priority_0 = TransactionPriority {
            transaction_nonce: 0,
            current_account_nonce: 0,
        };
        let priority_1 = TransactionPriority {
            transaction_nonce: 1,
            current_account_nonce: 0,
        };

        assert!(priority_0 > priority_1);
        assert!(priority_0 == priority_0);
        assert!(priority_1 < priority_0);
    }

    #[test]
    fn mempool_insert_pop() {
        let mut mempool = BasicMempool::new();

        let tx0 = get_mock_tx(0);
        let priority0 = TransactionPriority::new(0, 0).unwrap();
        mempool.insert(tx0.clone(), priority0.clone()).unwrap();

        let tx1 = get_mock_tx(1);
        let priority1 = TransactionPriority::new(1, 0).unwrap();
        mempool.insert(tx1.clone(), priority1.clone()).unwrap();

        assert!(priority0 > priority1);
        assert_eq!(mempool.len(), 2);

        let (tx, priority) = mempool.pop().unwrap();
        assert_eq!(
            tx.sha256_of_proto_encoding(),
            tx0.sha256_of_proto_encoding()
        );
        assert_eq!(priority, priority0);

        let (tx, priority) = mempool.pop().unwrap();
        assert_eq!(
            tx.sha256_of_proto_encoding(),
            tx1.sha256_of_proto_encoding()
        );
        assert_eq!(priority, priority1);
    }
}
