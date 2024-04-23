use std::{
    cmp::Ordering,
    collections::HashMap,
};

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use priority_queue::double_priority_queue::DoublePriorityQueue;

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// returns the number of transactions in the mempool
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub(crate) fn iter(&self) -> BasicMempoolIterator {
        BasicMempoolIterator {
            mempool_iter: self.queue.iter(),
            hash_to_tx: &self.hash_to_tx,
        }
    }

    #[must_use]
    pub(crate) fn get(&self, hash: &[u8; 32]) -> Option<&SignedTransaction> {
        self.hash_to_tx.get(hash)
    }

    /// inserts a transaction into the mempool
    pub(crate) fn insert(&mut self, tx: SignedTransaction, priority: TransactionPriority) {
        let hash = tx.sha256_of_proto_encoding();
        self.queue.push(hash, priority);
        self.hash_to_tx.insert(hash, tx);
    }

    /// inserts multiple transactions into the mempool
    pub(crate) fn insert_all(&mut self, txs: Vec<(SignedTransaction, TransactionPriority)>) {
        for (tx, priority) in txs {
            self.insert(tx, priority);
        }
    }

    /// changes the priority of a transaction in the mempool, if it exists
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

    /// pops the transaction with the highest priority from the mempool
    #[must_use]
    fn pop(&mut self) -> Option<(SignedTransaction, TransactionPriority)> {
        let (hash, priority) = self.queue.pop_max()?;
        let tx = self.hash_to_tx.remove(&hash)?;
        Some((tx, priority))
    }

    /// takes all transactions out of the mempool, leaving the mempool empty
    #[must_use]
    pub(crate) fn take_all(&mut self) -> Vec<(SignedTransaction, TransactionPriority)> {
        let mut txs = Vec::with_capacity(self.queue.len());
        while self.queue.peek_max().is_some() {
            txs.push(
                self.pop()
                    .expect("queue is not empty; we peeked and saw a value"),
            );
        }
        txs
    }

    /// iter which mutates the mempool by popping transactions out of its queue
    #[must_use]
    pub(crate) fn iter_pop(&mut self) -> BasicMempoolIterPop {
        BasicMempoolIterPop {
            mempool: self,
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

pub(crate) struct BasicMempoolIterator<'a> {
    mempool_iter: priority_queue::core_iterators::Iter<'a, [u8; 32], TransactionPriority>,
    hash_to_tx: &'a HashMap<[u8; 32], SignedTransaction>,
}

impl<'a> Iterator for BasicMempoolIterator<'a> {
    type Item = &'a SignedTransaction;

    fn next(&mut self) -> Option<Self::Item> {
        self.mempool_iter.next().map(|(hash, _)| {
            self.hash_to_tx
                .get(hash)
                .expect("if the hash is in the queue, it must be in the hash_to_tx map")
        })
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

pub(crate) struct BasicMempoolIterPop<'a> {
    mempool: &'a mut BasicMempool,
}

impl<'a> Iterator for BasicMempoolIterPop<'a> {
    type Item = (SignedTransaction, TransactionPriority);

    fn next(&mut self) -> Option<Self::Item> {
        self.mempool.pop()
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
