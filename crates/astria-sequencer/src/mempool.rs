use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use priority_queue::double_priority_queue::DoublePriorityQueue;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct TransactionPriority {
    nonce: u64,
}

struct BasicMempool {
    queue: DoublePriorityQueue<SignedTransaction, TransactionPriority>,
}
