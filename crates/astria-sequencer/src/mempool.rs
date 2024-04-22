use std::{
    cmp::Ordering,
    collections::HashMap,
};

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use priority_queue::double_priority_queue::DoublePriorityQueue;

#[derive(PartialEq, Eq)]
pub(crate) struct TransactionPriority {
    transaction_nonce: u32,
    current_account_nonce: u32,
}

impl PartialOrd for TransactionPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_priority = self.transaction_nonce - self.current_account_nonce;
        let other_priority = other.transaction_nonce - other.current_account_nonce;

        // we want to execute the lowest nonce first,
        // so lower nonce difference means higher priority
        if self_priority > other_priority {
            Some(Ordering::Less)
        } else if self_priority < other_priority {
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
    #[must_use]
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

/// BasicMempool is a simple mempool implementation that stores transactions in a priority queue
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

    #[must_use]
    pub(crate) fn iter(&mut self) -> BasicMempoolIterator {
        BasicMempoolIterator {
            mempool_iter: self.queue.iter(),
            mempool: self,
        }
    }

    // inserts a transaction into the mempool
    pub(crate) fn insert(&mut self, tx: SignedTransaction, priority: TransactionPriority) {
        let hash = tx.sha256_of_proto_encoding();
        self.queue.push(hash, priority);
        self.hash_to_tx.insert(hash, tx);
    }

    // changes the priority of a transaction in the mempool, if it exists
    pub(crate) fn change_priority(
        &mut self,
        tx: &SignedTransaction,
        new_priority: TransactionPriority,
    ) -> anyhow::Result<()> {
        if !self.hash_to_tx.contains_key(&tx.sha256_of_proto_encoding()) {
            return Err(anyhow::anyhow!("transaction not found in mempool"));
        }

        let hash = tx.sha256_of_proto_encoding();
        self.queue.change_priority(&hash, new_priority);
        Ok(())
    }

    // pops the transaction with the highest priority from the mempool
    #[must_use]
    fn pop(&mut self) -> Option<SignedTransaction> {
        let (hash, _) = self.queue.pop_max()?;
        self.hash_to_tx.remove(&hash)
    }

    // takes all transactions out of the mempool, leaving the mempool empty
    #[must_use]
    pub(crate) fn take_all(&mut self) -> Vec<SignedTransaction> {
        self.queue
            .iter()
            .filter_map(|(hash, _)| self.hash_to_tx.remove(hash))
            .collect()
    }
}

pub(crate) struct BasicMempoolIterator<'a> {
    mempool_iter: priority_queue::core_iterators::Iter<'a, [u8; 32], TransactionPriority>,
    mempool: &'a BasicMempool,
}

impl<'a> Iterator for BasicMempoolIterator<'a> {
    type Item = &'a SignedTransaction;

    fn next(&mut self) -> Option<Self::Item> {
        self.mempool_iter.next().map(|(hash, _)| {
            self.mempool
                .hash_to_tx
                .get(hash)
                .expect("if the hash is in the queue, it must be in the hash_to_tx map")
        })
    }
}

pub(crate) struct BasicMempoolIntoIterator {
    mempool: BasicMempool,
}

impl Iterator for BasicMempoolIntoIterator {
    type Item = SignedTransaction;

    fn next(&mut self) -> Option<Self::Item> {
        self.mempool.pop()
    }
}

impl IntoIterator for BasicMempool {
    type IntoIter = BasicMempoolIntoIterator;
    type Item = SignedTransaction;

    fn into_iter(self) -> Self::IntoIter {
        BasicMempoolIntoIterator {
            mempool: self,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
    }
}
