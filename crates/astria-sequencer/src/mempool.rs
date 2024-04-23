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

    /// inserts a transaction into the mempool
    pub(crate) fn insert(&mut self, tx: SignedTransaction, priority: TransactionPriority) {
        let hash = tx.sha256_of_proto_encoding();
        self.queue.push(hash, priority);
        self.hash_to_tx.insert(hash, tx);
    }

    /// pops the transaction with the highest priority from the mempool
    #[must_use]
    pub(crate) fn pop(&mut self) -> Option<(SignedTransaction, TransactionPriority)> {
        let (hash, priority) = self.queue.pop_max()?;
        let tx = self.hash_to_tx.remove(&hash)?;
        Some((tx, priority))
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
    use astria_core::{
        primitive::v1::RollupId,
        protocol::transaction::v1alpha1::{
            action::SequenceAction,
            TransactionParams,
            UnsignedTransaction,
        },
    };

    use super::*;
    use crate::{
        app::test_utils::get_alice_signing_key_and_address,
        asset::get_native_asset,
    };

    fn get_mock_tx() -> SignedTransaction {
        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let tx = UnsignedTransaction {
            params: TransactionParams {
                nonce: 0,
                chain_id: "test".to_string(),
            },
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                    data: vec![0x99],
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        tx.into_signed(&alice_signing_key)
    }

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

    #[test]
    fn mempool_insert_pop() {
        let mut mempool = BasicMempool::new();

        let tx1 = SignedTransaction::default();
        let priority1 = TransactionPriority::new(0, 0).unwrap();
        mempool.insert(tx1.clone(), priority1);

        let tx2 = SignedTransaction::default();
        let priority2 = TransactionPriority::new(1, 0).unwrap();
        mempool.insert(tx2.clone(), priority2);

        let (popped_tx2, popped_priority2) = mempool.pop().unwrap();
        assert_eq!(popped_tx2, tx2);
        assert_eq!(popped_priority2, priority2);

        let (popped_tx1, popped_priority1) = mempool.pop().unwrap();
        assert_eq!(popped_tx1, tx1);
        assert_eq!(popped_priority1, priority1);
    }
}
