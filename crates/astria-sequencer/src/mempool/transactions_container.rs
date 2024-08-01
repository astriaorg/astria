use std::{
    cmp::Ordering,
    collections::{
        BTreeMap,
        HashMap,
        HashSet,
    },
    fmt,
    future::Future,
    sync::Arc,
};

use anyhow::Context;
use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use priority_queue::PriorityQueue;
use tokio::time::{
    Duration,
    Instant,
};

use super::RemovalReason;

/// [`TimemarkedTransaction`] is a wrapper around a signed transaction
/// used to keep track of when that transaction was first seen in the mempool.
///
/// Note: `PartialEq` was implemented for this struct to only take into account
/// the signed transaction's hash.  
#[derive(Debug)]
pub(crate) struct TimemarkedTransaction {
    signed_tx: SignedTransaction,
    tx_hash: [u8; 32],
    time_first_seen: Instant,
    address: [u8; 20],
}

impl TimemarkedTransaction {
    pub(crate) fn new(signed_tx: SignedTransaction) -> Self {
        Self {
            tx_hash: signed_tx.sha256_of_proto_encoding(),
            address: signed_tx.verification_key().address_bytes(),
            signed_tx,
            time_first_seen: Instant::now(),
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
            time_first_seen: self.time_first_seen,
        })
    }

    pub(crate) fn time_first_seen(&self) -> Instant {
        self.time_first_seen
    }

    pub(crate) fn signed_tx(&self) -> &SignedTransaction {
        &self.signed_tx
    }

    pub(crate) fn tx_hash(&self) -> [u8; 32] {
        self.tx_hash
    }

    pub(crate) fn address(&self) -> &[u8; 20] {
        &self.address
    }
}

/// Only consider `self.tx_hash` for equality. This is consistent with the impl for std `Hash`.
impl PartialEq for TimemarkedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash
    }
}

impl Eq for TimemarkedTransaction {}

/// Only consider `self.tx_hash` when hashing. This is consistent with the impl for equality.
impl std::hash::Hash for TimemarkedTransaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tx_hash.hash(state);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransactionPriority {
    nonce_diff: u32,
    time_first_seen: Instant,
}

impl PartialEq for TransactionPriority {
    fn eq(&self, other: &Self) -> bool {
        self.nonce_diff == other.nonce_diff && self.time_first_seen == other.time_first_seen
    }
}

impl Eq for TransactionPriority {}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // we want to first order by nonce difference
        // lower nonce diff means higher priority
        let nonce_diff = self.nonce_diff.cmp(&other.nonce_diff).reverse();

        // then by timestamp if equal
        if nonce_diff == Ordering::Equal {
            // lower timestamp means higher priority
            return self.time_first_seen.cmp(&other.time_first_seen).reverse();
        }
        nonce_diff
    }
}

impl PartialOrd for TransactionPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// [`BuilderQueue`] is a typed used to order transactions by the difference between a transaction
/// nonce and the account nonce then by the time that a transaction was first seen by the mempool.
pub(crate) type BuilderQueue = PriorityQueue<Arc<TimemarkedTransaction>, TransactionPriority>;

/// [`AccountTransactionContainer`] is a container used for managing transactions belonging
/// to a single account.
///
/// The `strict` mode defines if transaction nonces are allowed to be gapped or not. The
/// `size_limit` is only enfored when the `strict` mode is false.
#[derive(Clone, Debug)]
struct AccountTransactionContainer {
    txs: BTreeMap<u32, Arc<TimemarkedTransaction>>, // tracked transactions
    strict: bool,                                   // if nonce gaps are allowed or not
    size_limit: usize,                              /* max number of transactions stored, only
                                                     * enforced if not strict */
}

#[derive(Debug, Clone)]
pub(crate) enum InsertionError {
    AlreadyPresent,
    NonceTooLow,
    NonceTaken,
    NonceGap,
    AccountSizeLimit,
}

impl fmt::Display for InsertionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InsertionError::AlreadyPresent => write!(f, "AlreadyPresent"),
            InsertionError::NonceTooLow => write!(f, "NonceTooLow"),
            InsertionError::NonceTaken => write!(f, "NonceTaken"),
            InsertionError::NonceGap => write!(f, "NonceGap"),
            InsertionError::AccountSizeLimit => write!(f, "AccountSizeLimit"),
        }
    }
}

impl AccountTransactionContainer {
    fn new(strict: bool, size_limit: usize) -> Self {
        AccountTransactionContainer {
            txs: BTreeMap::<u32, Arc<TimemarkedTransaction>>::new(),
            strict,
            size_limit,
        }
    }

    /// Adds transaction to the container. Note: does NOT allow for nonce replacement.
    /// Will fail if `strict` is true and adding the transaction would create a nonce gap.
    ///
    /// `current_account_nonce` should be the nonce that the current accounts state is at.
    /// Note: if the account `current_account_nonce` ever decreases, this is a logic error
    /// and could mess up the validity of strict containers.
    fn add(
        &mut self,
        ttx: Arc<TimemarkedTransaction>,
        current_account_nonce: u32,
    ) -> anyhow::Result<(), InsertionError> {
        if !self.strict && self.txs.len() >= self.size_limit {
            return Err(InsertionError::AccountSizeLimit);
        }

        if ttx.signed_tx().nonce() < current_account_nonce {
            return Err(InsertionError::NonceTooLow);
        }

        if self.txs.contains_key(&ttx.signed_tx.nonce()) {
            return Err(InsertionError::NonceTaken);
        }

        // ensure that if strict mode is on that the previous nonce is already in the list
        if self.strict
            && ttx.signed_tx.nonce() != 0
            && ttx.signed_tx.nonce() != current_account_nonce
            && !self.txs.contains_key(
                &ttx.signed_tx
                    .nonce()
                    .checked_sub(1)
                    .expect("Error subtracting from non zero nonce"),
            )
        {
            // nonce is gapped
            return Err(InsertionError::NonceGap);
        }

        // add to internal structure
        self.txs.insert(ttx.signed_tx.nonce(), ttx);

        Ok(())
    }

    /// Removes transaction from the container. If `remove_higher` is true, will
    /// remove all higher nonces from account as well. Returns a vector of removed transactions.
    fn remove(&mut self, nonce: u32, remove_higher: bool) -> Vec<Arc<TimemarkedTransaction>> {
        let mut result = Vec::<Arc<TimemarkedTransaction>>::new();
        if !self.txs.contains_key(&nonce) {
            // doesn't contain requested nonce
            return result;
        }

        if remove_higher {
            // remove from end till all higher nonces removed
            loop {
                let (key, value) = self
                    .txs
                    .pop_last()
                    .expect("Error popping values that should exist");
                result.push(value);
                if key == nonce {
                    break;
                }
            }
        } else {
            // remove single element
            result.push(
                self.txs
                    .remove(&nonce)
                    .expect("Error removing value that should exist"),
            );
        }
        result
    }

    /// Returns a copy of all of the contained transactions.
    fn all_transactions_copy(&self) -> Vec<Arc<TimemarkedTransaction>> {
        self.txs.clone().into_values().collect()
    }

    /// Returns all transactions, consuming them in the process.
    fn pop_all_transactions(&mut self) -> Vec<Arc<TimemarkedTransaction>> {
        let mut popped_txs = Vec::<Arc<TimemarkedTransaction>>::new();
        while let Some((_, tx)) = self.txs.pop_first() {
            popped_txs.push(tx);
        }
        popped_txs
    }

    /// Returns contiguous transactions from front of queue starting from target nonce, removing the
    /// transactions in the process. Used when need to promote transactions from parked to
    /// pending.
    ///
    /// Note: this function only operates on the front of the queue. If the target nonce
    /// is not at the front, nothing will be returned.
    fn pop_front_multiple(&mut self, mut target_nonce: u32) -> Vec<Arc<TimemarkedTransaction>> {
        assert!(!self.strict, "shouldn't be popping from strict queue");
        let mut popped_txs = Vec::<Arc<TimemarkedTransaction>>::new();

        while let Some((nonce, _)) = self.txs.first_key_value() {
            if *nonce == target_nonce {
                let (_, tx) = self
                    .txs
                    .pop_first()
                    .expect("popped value should exist in pop_front");
                popped_txs.push(tx);
                target_nonce = target_nonce
                    .checked_add(1)
                    .expect("failed while incrementing nonce");
            } else {
                // not target nonce
                break;
            }
        }
        popped_txs
    }

    /// Pops the lowest transaction.
    fn pop_front_single(&mut self) -> Option<Arc<TimemarkedTransaction>> {
        if let Some((_, tx)) = self.txs.pop_first() {
            Some(tx)
        } else {
            None
        }
    }

    /// Returns the highest nonce value.
    fn peek_end(&self) -> Option<u32> {
        if let Some((nonce, _)) = self.txs.last_key_value() {
            Some(*nonce)
        } else {
            None
        }
    }

    /// Returns a copy of the lowest transaction.
    fn peek_front(&self) -> Option<Arc<TimemarkedTransaction>> {
        if let Some((_, tx)) = self.txs.first_key_value() {
            Some(tx.clone())
        } else {
            None
        }
    }

    /// Remove transactions below the current account nonce. Used for
    /// clearing out stale nonces. Returns the transactions that were removed.
    fn fast_forward(&mut self, current_account_nonce: u32) -> Vec<Arc<TimemarkedTransaction>> {
        let mut removed_txs = Vec::<Arc<TimemarkedTransaction>>::new();
        while let Some((nonce, _)) = self.txs.first_key_value() {
            if *nonce < current_account_nonce {
                let (_, tx) = self
                    .txs
                    .pop_first()
                    .expect("popped value should exist in fast_forward");
                removed_txs.push(tx);
            } else {
                // cleared out stale nonces
                break;
            }
        }
        removed_txs
    }

    /// Returns the number of transactions in container.
    fn size(&self) -> usize {
        self.txs.len()
    }
}

/// [`AccountTransactionContainer`] is a container used for mananging transactions for
/// multiple accounts.
///
/// The `strict` mode defines if transaction nonces are allowed to be gapped or not for the managed
/// accounts. The `size_limit` is only enfored when the `strict` mode is false.
#[derive(Clone, Debug)]
pub(crate) struct TransactionContainer {
    accounts: HashMap<[u8; 20], AccountTransactionContainer>,
    all: HashSet<[u8; 32]>,
    strict: bool,
    size_limit: usize,
    tx_ttl: Duration,
}

impl TransactionContainer {
    pub(crate) fn new(strict: bool, size_limit: usize, tx_ttl: Duration) -> Self {
        assert!(
            (!strict && size_limit != 0) || (strict && size_limit == 0),
            "pending shouldn't have size restrictions and parked should"
        );
        TransactionContainer {
            accounts: HashMap::<[u8; 20], AccountTransactionContainer>::new(),
            all: HashSet::<[u8; 32]>::new(),
            strict,
            size_limit,
            tx_ttl,
        }
    }

    #[cfg(test)]
    /// Returns the number of transactions in the container.
    pub(crate) fn size(&self) -> usize {
        self.all.len()
    }

    /// Returns the highest nonce for an account.
    pub(crate) fn pending_nonce(&self, address: [u8; 20]) -> Option<u32> {
        if let Some(account) = self.accounts.get(&address) {
            account.peek_end()
        } else {
            None
        }
    }

    /// Adds the transaction to the container. If failed,
    /// returns the reason why.
    ///
    /// `current_account_nonce` should be the current nonce
    /// of the account associated with the transaction. If this
    /// ever decreases, the strict containers could become messed up.
    pub(crate) fn add(
        &mut self,
        ttx: Arc<TimemarkedTransaction>,
        current_account_nonce: u32,
    ) -> anyhow::Result<(), InsertionError> {
        // already tracked
        if self.all.contains(&ttx.tx_hash()) {
            return Err(InsertionError::AlreadyPresent);
        }

        // create account map if necessary
        let mut account_created = false;
        if !self.accounts.contains_key(ttx.address()) {
            account_created = true;
            self.accounts.insert(
                *ttx.address(),
                AccountTransactionContainer::new(self.strict, self.size_limit),
            );
        }

        // try to add transaction
        let address_cache = *ttx.address();
        let hash_cache = ttx.tx_hash();
        let success = self
            .accounts
            .get_mut(ttx.address())
            .expect("AccountTransactionsContainer for account should exist")
            .add(ttx, current_account_nonce);

        if success.is_ok() {
            // add to all tracked if successfully added to account
            self.all.insert(hash_cache);
        } else if account_created {
            // remove freshly created account if insertion failed
            self.accounts.remove(&address_cache);
        }

        success
    }

    /// Removes the target transaction and any transactions with higher
    /// nonces if `remove_higher` is set to true. Returns all removed transactions.
    ///
    /// Note: operates on the account<>nonce pair of the target transaction instead of the
    /// transaction's hash.
    pub(crate) fn remove(
        &mut self,
        ttx: &Arc<TimemarkedTransaction>,
        remove_higher: bool,
    ) -> Vec<Arc<TimemarkedTransaction>> {
        let address = ttx.signed_tx.verification_key().address_bytes();

        // return if no tracked account
        if !self.accounts.contains_key(&address) {
            return Vec::<Arc<TimemarkedTransaction>>::new();
        }

        // remove transactions
        let removed_txs = self
            .accounts
            .get_mut(&address)
            .expect("AccountTransactionsContainer for account should exist")
            .remove(ttx.signed_tx.nonce(), remove_higher);

        // remove transactions from all tracked
        for tx in &removed_txs {
            self.all.remove(&tx.tx_hash);
        }

        // remove account if out of transactions
        if self
            .accounts
            .get(&address)
            .expect("AccountTransactionsContainer for account should still exist")
            .size()
            == 0
        {
            self.accounts.remove(&address);
        }

        removed_txs
    }

    /// Removes all of the transactions from an account and deletes the account entry.
    /// Returns the removed transactions.
    pub(crate) fn clear_account(&mut self, address: &[u8; 20]) -> Vec<Arc<TimemarkedTransaction>> {
        let mut removed_txs = Vec::<Arc<TimemarkedTransaction>>::new();
        if let Some(account) = self.accounts.get_mut(address) {
            removed_txs.append(&mut account.pop_all_transactions());

            // remove from all
            for tx in &removed_txs {
                self.all.remove(&tx.tx_hash());
            }
            // remove account
            self.accounts.remove(address);
        }
        removed_txs
    }

    /// Cleans all of the accounts in the container. Will remove any transactions with stale nonces
    /// and will evict all transactions from accounts whose lowest transaction has expired.
    ///
    /// Returns all transactions that have been removed with the reason why they have been removed.
    pub(crate) async fn clean_accounts<F, O>(
        &mut self,
        current_account_nonce_getter: &F,
    ) -> anyhow::Result<Vec<(Arc<TimemarkedTransaction>, RemovalReason)>>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        // currently just removes stale nonces and will clear accounts if the
        // transactions are older than the TTL
        let mut accounts_to_remove = Vec::<[u8; 20]>::new();
        let mut removed_txs = Vec::<(Arc<TimemarkedTransaction>, RemovalReason)>::new();
        for (address, account_txs) in &mut self.accounts {
            // check if first tx is older than the TTL, if so, remove all transactions
            if let Some(first_tx) = account_txs.peek_front() {
                if first_tx.time_first_seen().elapsed() > self.tx_ttl {
                    // first is stale, rest popped for invalidation
                    removed_txs.push((
                        account_txs
                            .pop_front_single()
                            .expect("first tx should exist"),
                        RemovalReason::Expired,
                    ));
                    for tx in account_txs.pop_all_transactions() {
                        println!("other nonce: {}", tx.signed_tx().nonce());
                        removed_txs.push((tx, RemovalReason::LowerNonceInvalidated));
                    }
                } else {
                    // clean to newest nonce
                    let current_account_nonce = current_account_nonce_getter(*address)
                        .await
                        .context("failed to fetch account nonce for fast forward")?;
                    for tx in account_txs.fast_forward(current_account_nonce) {
                        removed_txs.push((tx, RemovalReason::NonceStale));
                    }
                }
            }

            if account_txs.size() == 0 {
                accounts_to_remove.push(*address);
            }
        }

        // remove empty accounts
        for account in accounts_to_remove {
            // remove empty accounts
            self.accounts.remove(&account);
        }

        // untrack transactions
        for (tx, _) in &removed_txs {
            self.all.remove(&tx.tx_hash());
        }

        Ok(removed_txs)
    }

    /// Removes and returns transactions that are lower than or equal to the current nonce of
    /// accounts. This is helpful when needing to promote transactions from parked to pending
    /// during mempool maintenance.
    pub(crate) async fn find_promotables<F, O>(
        &mut self,
        current_account_nonce_getter: &F,
    ) -> anyhow::Result<Vec<Arc<TimemarkedTransaction>>>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        assert!(!self.strict, "should not be promoting from pending");
        let mut accounts_to_remove = Vec::<[u8; 20]>::new();
        let mut promoted_txs = Vec::<Arc<TimemarkedTransaction>>::new();

        for (address, account_txs) in &mut self.accounts {
            let current_account_nonce = current_account_nonce_getter(*address)
                .await
                .context("failed to fetch account nonce for pop front")?;

            // find transactions that can be promoted
            // note: can use current account nonce as target because this logic
            // is only handling the case where transactions we didn't have in our
            // local mempool were ran that would enable the parked transactions to
            // be valid
            promoted_txs.append(&mut account_txs.pop_front_multiple(current_account_nonce));

            if account_txs.size() == 0 {
                accounts_to_remove.push(*address);
            }
        }

        // remove empty accounts
        for account in accounts_to_remove {
            // remove empty accounts
            self.accounts.remove(&account);
        }

        // untrack transactions
        for tx in &promoted_txs {
            self.all.remove(&tx.tx_hash());
        }

        Ok(promoted_txs)
    }

    /// Removes and returns the transactions from the front of an account, similar to
    /// `find_promotables()`. Useful for when needing to promote transactions from a specific
    /// account instead of all accounts.
    pub(crate) fn pop_front_account(
        &mut self,
        account: &[u8; 20],
        target_nonce: u32,
    ) -> Vec<Arc<TimemarkedTransaction>> {
        if let Some(account_txs) = self.accounts.get_mut(account) {
            let removed_txs = account_txs.pop_front_multiple(target_nonce);
            for tx in &removed_txs {
                // remove popped transactions from all
                self.all.remove(&tx.tx_hash());
            }

            if account_txs.size() == 0 {
                // remove empty account
                self.accounts.remove(account);
            }

            removed_txs
        } else {
            Vec::<Arc<TimemarkedTransaction>>::new()
        }
    }

    /// Returns a copy of transactions sorted by nonce difference and then time first seen.
    pub(crate) async fn builder_queue<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<BuilderQueue>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        assert!(
            self.strict,
            "shouldn't be calling build on gapped container"
        );

        let mut builder_queue = BuilderQueue::new();
        for (address, account_txs) in &self.accounts {
            let current_account_nonce = current_account_nonce_getter(*address)
                .await
                .context("failed to fetch account nonce for builder queue")?;

            let txs = account_txs.all_transactions_copy();
            for tx in txs {
                match tx.priority(current_account_nonce) {
                    Ok(tx_priority) => {
                        builder_queue.push(tx.clone(), tx_priority);
                    }
                    Err(_) => continue, // mempool could be off due to node connectivity issues
                }
            }
        }
        Ok(builder_queue)
    }
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

    use astria_core::crypto::SigningKey;

    use super::*;
    use crate::app::test_utils::get_mock_tx_parameterized;

    const STRICT_SIZE_LIMIT: usize = 0;
    const UNSTRICT_SIZE_LIMIT: usize = 15;

    #[test]
    fn transaction_priority_should_error_if_invalid() {
        let ttx =
            TimemarkedTransaction::new(get_mock_tx_parameterized(0, &[1; 32].into(), [1; 32]));
        let priority = ttx.priority(1);

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
    fn transaction_priority_comparisons_should_be_consistent_nonce_diff() {
        let instant = Instant::now();

        let high = TransactionPriority {
            nonce_diff: 0,
            time_first_seen: instant,
        };
        let low = TransactionPriority {
            nonce_diff: 1,
            time_first_seen: instant,
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

    // From https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
    #[test]
    // allow: we want explicit assertions here to match the documented expected behavior.
    #[allow(clippy::nonminimal_bool)]
    fn transaction_priority_comparisons_should_be_consistent_time_gap() {
        let high = TransactionPriority {
            nonce_diff: 0,
            time_first_seen: Instant::now(),
        };
        let low = TransactionPriority {
            nonce_diff: 0,
            time_first_seen: Instant::now() + Duration::from_micros(10),
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
    fn timemarked_tx_hash_and_eq_should_be_consistent() {
        // Check timemarked txs compare equal if and only if their tx hashes are equal.
        let signed_tx_0_a = get_mock_tx_parameterized(0, &[1; 32].into(), [1; 32]);
        let tx0 = TimemarkedTransaction {
            tx_hash: [0; 32],
            signed_tx: signed_tx_0_a.clone(),
            address: signed_tx_0_a.verification_key().address_bytes(),

            time_first_seen: Instant::now(),
        };
        let signed_tx_0_b = get_mock_tx_parameterized(1, &[1; 32].into(), [1; 32]);
        let other_tx0 = TimemarkedTransaction {
            tx_hash: [0; 32],
            signed_tx: signed_tx_0_b.clone(),
            address: signed_tx_0_b.verification_key().address_bytes(),

            time_first_seen: Instant::now(),
        };
        let signed_tx_1 = get_mock_tx_parameterized(1, &[1; 32].into(), [1; 32]);
        let tx1 = TimemarkedTransaction {
            tx_hash: [1; 32],
            signed_tx: signed_tx_1.clone(),
            address: signed_tx_1.verification_key().address_bytes(),
            time_first_seen: Instant::now(),
        };
        assert!(tx0 == other_tx0);
        assert!(tx0 != tx1);

        // Check timemarked txs' std hashes compare equal if and only if their tx hashes are equal.
        let std_hash = |ttx: &TimemarkedTransaction| -> u64 {
            let mut hasher = std::hash::DefaultHasher::new();
            ttx.hash(&mut hasher);
            hasher.finish()
        };
        assert!(std_hash(&tx0) == std_hash(&other_tx0));
        assert!(std_hash(&tx0) != std_hash(&tx1));
    }

    #[test]
    fn account_trasaction_container_non_strict_add() {
        let mut account_container = AccountTransactionContainer::new(false, UNSTRICT_SIZE_LIMIT);

        // transactions to add
        let ttx_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_5 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            5,
            &[1; 32].into(),
            [1; 32],
        )));

        let current_account_nonce = 2;
        assert!(matches!(
            account_container.add(ttx_3.clone(), current_account_nonce),
            Ok(())
        ));
        assert!(matches!(
            account_container.add(ttx_3, current_account_nonce),
            Err(InsertionError::NonceTaken)
        ));

        // add gapped transaction
        assert!(matches!(
            account_container.add(ttx_5, current_account_nonce),
            Ok(())
        ));

        // fail adding too low nonce
        assert!(matches!(
            account_container.add(ttx_1, current_account_nonce),
            Err(InsertionError::NonceTooLow)
        ));
    }

    #[test]
    fn account_trasaction_container_non_strict_size_limit() {
        let mut account_container = AccountTransactionContainer::new(false, 2);

        // transactions to add
        let ttx_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_5 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            5,
            &[1; 32].into(),
            [1; 32],
        )));

        let current_account_nonce = 0;
        assert!(matches!(
            account_container.add(ttx_3.clone(), current_account_nonce),
            Ok(())
        ));
        assert!(matches!(
            account_container.add(ttx_5, current_account_nonce),
            Ok(())
        ));

        // fail with size limit hit
        assert!(matches!(
            account_container.add(ttx_1, current_account_nonce),
            Err(InsertionError::AccountSizeLimit)
        ));
    }

    #[test]
    fn account_trasaction_container_strict_add() {
        let mut account_container = AccountTransactionContainer::new(true, STRICT_SIZE_LIMIT);

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));

        let current_account_nonce = 1;

        // too low nonces not added
        assert!(matches!(
            account_container.add(ttx_0, current_account_nonce),
            Err(InsertionError::NonceTooLow)
        ));

        // add ok
        assert!(matches!(
            account_container.add(ttx_1.clone(), current_account_nonce),
            Ok(())
        ));
        assert!(matches!(
            account_container.add(ttx_1, current_account_nonce),
            Err(InsertionError::NonceTaken)
        ));

        // gapped transaction not allowed
        assert!(matches!(
            account_container.add(ttx_3, current_account_nonce),
            Err(InsertionError::NonceGap)
        ));

        // can add consecutive
        assert!(matches!(
            account_container.add(ttx_2, current_account_nonce),
            Ok(())
        ));
    }

    #[test]
    fn account_trasaction_container_remove() {
        let mut account_container = AccountTransactionContainer::new(true, STRICT_SIZE_LIMIT);

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));

        account_container.add(ttx_0, 0).unwrap();
        account_container.add(ttx_1, 0).unwrap();
        account_container.add(ttx_2, 0).unwrap();
        account_container.add(ttx_3, 0).unwrap();

        // non-remove-higher remove ok
        assert_eq!(
            account_container.remove(1, false).len(),
            1,
            "tranasction should've been removed"
        );
        assert_eq!(account_container.size(), 3);

        // remove same again return nothing
        assert_eq!(
            account_container.remove(1, true).len(),
            0,
            "no transaction should be removed"
        );

        // remove from end will only remove end
        assert_eq!(
            account_container.remove(3, true).len(),
            1,
            "only one transaction should've been removed"
        );
        assert_eq!(account_container.size(), 2);

        // remove higher from start will remove all
        assert_eq!(
            account_container.remove(0, true).len(),
            2,
            "two transactions should've been removed"
        );
        assert_eq!(account_container.size(), 0);
    }

    #[test]
    fn account_trasaction_pop_front_multiple() {
        let mut account_container = AccountTransactionContainer::new(false, UNSTRICT_SIZE_LIMIT);

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_4 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            4,
            &[1; 32].into(),
            [1; 32],
        )));

        account_container.add(ttx_0, 0).unwrap();
        account_container.add(ttx_2, 0).unwrap();
        account_container.add(ttx_3, 0).unwrap();
        account_container.add(ttx_4, 0).unwrap();

        // lowest nonce not target nonce is noop
        assert_eq!(
            account_container.pop_front_multiple(2).len(),
            0,
            "no transaction should've been removed"
        );
        assert_eq!(account_container.size(), 4);

        // will remove single value
        assert_eq!(
            account_container.pop_front_multiple(0).len(),
            1,
            "single transaction should've been returned"
        );
        assert_eq!(account_container.size(), 3);

        // will remove multiple values
        assert_eq!(
            account_container.pop_front_multiple(2).len(),
            3,
            "multiple transaction should've been returned"
        );
        assert_eq!(account_container.size(), 0);
    }

    #[test]
    fn account_trasaction_peek_end() {
        let mut account_container = AccountTransactionContainer::new(false, UNSTRICT_SIZE_LIMIT);

        // no transactions ok
        assert_eq!(
            account_container.peek_end(),
            None,
            "no transactions will return None"
        );

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));

        account_container.add(ttx_0, 0).unwrap();
        account_container.add(ttx_2, 0).unwrap();

        // will return last transaction
        assert_eq!(
            account_container.peek_end(),
            Some(2),
            "highest nonce should be returned"
        );
    }

    #[test]
    fn account_trasaction_peek_front() {
        let mut account_container = AccountTransactionContainer::new(false, UNSTRICT_SIZE_LIMIT);

        // no transactions ok
        assert_eq!(
            account_container.peek_front(),
            None,
            "no transactions will return None"
        );

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));

        account_container.add(ttx_0.clone(), 0).unwrap();
        account_container.add(ttx_2, 0).unwrap();

        // will return last transaction
        assert_eq!(
            account_container.peek_front(),
            Some(ttx_0),
            "lowest transaction should be returned"
        );
    }

    #[test]
    fn account_trasaction_fast_forward() {
        let mut account_container = AccountTransactionContainer::new(false, UNSTRICT_SIZE_LIMIT);

        // transactions to add
        let ttx_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &[1; 32].into(),
            [1; 32],
        )));
        let ttx_4 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            4,
            &[1; 32].into(),
            [1; 32],
        )));

        account_container.add(ttx_0, 0).unwrap();
        account_container.add(ttx_2, 0).unwrap();
        account_container.add(ttx_3, 0).unwrap();
        account_container.add(ttx_4, 0).unwrap();

        // matching nonce will not be removed
        assert_eq!(
            account_container.fast_forward(0).len(),
            0,
            "no transaction should've been removed"
        );
        assert_eq!(account_container.size(), 4);

        // fast forwarding to non existing middle nonce ok
        assert_eq!(
            account_container.fast_forward(1).len(),
            1,
            "one transaction should've been removed"
        );
        assert_eq!(account_container.size(), 3);

        // fast forwarding to existing nonce ok
        assert_eq!(
            account_container.fast_forward(3).len(),
            1,
            "one transaction should've been removed"
        );
        assert_eq!(account_container.size(), 2);

        // fast forwarding to much higher nonce ok
        assert_eq!(
            account_container.fast_forward(10).len(),
            2,
            "two transaction should've been removed"
        );
        assert_eq!(account_container.size(), 0);
    }

    #[test]
    fn transaction_container_pending_nonces() {
        let mut transaction_container =
            TransactionContainer::new(false, UNSTRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add for account 0
        let ttx_s0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_0,
            [1; 32],
        )));

        transaction_container.add(ttx_s0_0, 0).unwrap();
        transaction_container.add(ttx_s0_2, 0).unwrap();

        // empty account returns zero
        assert_eq!(
            transaction_container.pending_nonce(signing_address_1),
            None,
            "empty account should return None"
        );

        // non empty account returns highest nonce
        assert_eq!(
            transaction_container.pending_nonce(signing_address_0),
            Some(2),
            "should return highest nonce"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn transaction_container_add() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s0_2_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_1,
            [1; 32],
        )));

        // transactions to add for account 1

        // initially no accounts should exist
        assert_eq!(
            transaction_container.accounts.len(),
            0,
            "no accounts should exist at first"
        );

        // adding too low nonce shouldn't create account
        assert!(
            matches!(
                transaction_container.add(ttx_s0_0_0.clone(), 1),
                Err(InsertionError::NonceTooLow)
            ),
            "shouldn't be able to add nonce too low transaction"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            0,
            "failed adds to new accounts shouldn't create account"
        );

        // add one transaction
        assert!(
            matches!(transaction_container.add(ttx_s0_0_0.clone(), 0), Ok(())),
            "should be able to add transaction"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "one account should exist"
        );

        // readding transaction should fail
        assert!(
            matches!(
                transaction_container.add(ttx_s0_0_0, 0),
                Err(InsertionError::AlreadyPresent)
            ),
            "readding same transaction should fail"
        );

        // nonce replacement fails
        assert!(
            matches!(
                transaction_container.add(ttx_s0_0_1, 0),
                Err(InsertionError::NonceTaken)
            ),
            "nonce replacement not supported"
        );

        // nonce gaps not supported
        assert!(
            matches!(
                transaction_container.add(ttx_s0_2_0, 0),
                Err(InsertionError::NonceGap)
            ),
            "gapped nonces in strict not allowed"
        );

        // add transactions for account 2
        assert!(
            matches!(transaction_container.add(ttx_s1_0_0, 0), Ok(())),
            "should be able to add transaction to second account"
        );

        // check internal structures
        assert_eq!(
            transaction_container.accounts.len(),
            2,
            "two accounts should exist"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_0)
                .unwrap()
                .size(),
            1,
            "one transaction should be in the original account"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            1,
            "one transaction should be in the second account"
        );
        assert_eq!(
            transaction_container.size(),
            2,
            "should only have two transactions tracked"
        );
    }

    #[test]
    fn transaction_container_remove() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s0_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_1,
            [1; 32],
        )));

        // remove on empty returns zero
        assert_eq!(
            transaction_container.remove(&ttx_s0_0, true).len(),
            0,
            "zero transactions should be removed from non existing accounts"
        );

        // add transactions
        transaction_container.add(ttx_s0_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_2.clone(), 0).unwrap();

        // remove with remove_higher to false should only remove one
        assert_eq!(
            transaction_container.remove(&ttx_s0_0, false).len(),
            1,
            "single transactions should be removed when remove_higher is false"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_0)
                .unwrap()
                .size(),
            2,
            "two transactions should be in the original account"
        );

        // remove with remove_higher set to true should remove rest
        assert_eq!(
            transaction_container.remove(&ttx_s0_1, true).len(),
            2,
            "rest of transactions should be removed when remove_higher is true and targeting \
             bottom nonce"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "empty account should be removed"
        );
        assert_eq!(
            transaction_container.size(),
            3,
            "should only have three transactions tracked"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            3,
            "other account should be untouched"
        );
    }

    #[test]
    fn transaction_container_clear_account() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_1,
            [1; 32],
        )));

        // clear all on empty returns zero
        assert_eq!(
            transaction_container
                .clear_account(&signing_address_0)
                .len(),
            0,
            "zero transactions should be removed from clearing non existing accounts"
        );

        // add transactions
        transaction_container.add(ttx_s0_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_0.clone(), 0).unwrap();

        // clear should return all transactions
        assert_eq!(
            transaction_container
                .clear_account(&signing_address_0)
                .len(),
            2,
            "all transactions should be returned from clearing account"
        );

        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "empty account should be removed"
        );
        assert_eq!(
            transaction_container.size(),
            1,
            "should only have one transaction tracked"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            1,
            "other account should be untouched"
        );
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn transaction_container_clean_accounts_nonce() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();
        let signing_key_2 = SigningKey::from([3; 32]);
        let signing_address_2 = signing_key_2.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s0_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s2_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_2,
            [1; 32],
        )));
        let ttx_s2_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_2,
            [1; 32],
        )));
        let ttx_s2_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_2,
            [1; 32],
        )));

        // add transactions
        transaction_container.add(ttx_s0_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s2_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s2_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s2_2.clone(), 0).unwrap();

        // current nonce getter
        // should pop none from signing_address_0, one from signing_address_1, and all from
        // signing_address_2
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                return Ok(0);
            }
            if address == signing_address_1 {
                return Ok(1);
            }
            if address == signing_address_2 {
                return Ok(4);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        let removed_txs = transaction_container
            .clean_accounts(&current_account_nonce_getter)
            .await
            .unwrap();

        assert_eq!(
            removed_txs.len(),
            4,
            "four transactions should've been popped"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            2,
            "empty accounts should be removed"
        );
        assert_eq!(
            transaction_container.size(),
            5,
            "5 transactions should be remaining from original 9"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_0)
                .unwrap()
                .size(),
            3
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            2
        );
        for (_, reason) in removed_txs {
            assert!(
                matches!(reason, RemovalReason::NonceStale),
                "removal reason should be stale nonce"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn transaction_container_clean_accounts_expired_transactions() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_0,
            [1; 32],
        )));

        // pass time to make first transaction stale
        tokio::time::advance(Duration::from_secs(5)).await;

        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_0 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            0,
            &signing_key_1,
            [1; 32],
        )));

        // add transactions
        transaction_container.add(ttx_s0_0.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_0.clone(), 0).unwrap();

        // current nonce getter
        // all nonces should be valid
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                return Ok(0);
            }
            if address == signing_address_1 {
                return Ok(0);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        let mut removed_txs = transaction_container
            .clean_accounts(&current_account_nonce_getter)
            .await
            .unwrap();

        assert_eq!(
            removed_txs.len(),
            2,
            "two transactions should've been popped"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "empty accounts should be removed"
        );
        assert_eq!(
            transaction_container.size(),
            1,
            "1 transaction should be remaining from original 3"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            1,
            "not expired account should have expected transactions"
        );

        // check removal reasons
        let first_pop = removed_txs.pop().expect("should have tx to pop");
        assert_eq!(
            first_pop.0.tx_hash(),
            ttx_s0_1.tx_hash(),
            "first pop should be last pushed tx, which should be the second tx"
        );
        assert!(
            matches!(first_pop.1, RemovalReason::LowerNonceInvalidated),
            "first transaction's removal reason should be lower nonce invalidation"
        );
        let second_pop = removed_txs.pop().expect("should have another tx to pop");
        assert_eq!(
            second_pop.0.tx_hash(),
            ttx_s0_0.tx_hash(),
            "second pop should be first added tx, to verify for next check"
        );
        assert!(
            matches!(second_pop.1, RemovalReason::Expired),
            "first transaction's removal reason should be expiration"
        );
    }

    #[tokio::test]
    async fn transaction_container_find_promotables() {
        let mut transaction_container =
            TransactionContainer::new(false, UNSTRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s0_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s0_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &signing_key_0,
            [2; 32],
        )));
        let ttx_s1_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_4 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            4,
            &signing_key_1,
            [1; 32],
        )));

        // add transactions
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s0_3.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_4.clone(), 0).unwrap();

        // current nonce getter
        // should pop all from signing_address_0 and two from signing_address_1
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                return Ok(1);
            }
            if address == signing_address_1 {
                return Ok(1);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        assert_eq!(
            transaction_container
                .find_promotables(&current_account_nonce_getter)
                .await
                .expect("find promotables should work")
                .len(),
            5,
            "five transactions should've been popped"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "empty accounts should be removed"
        );
        assert_eq!(
            transaction_container.size(),
            1,
            "1 transactions should be remaining from original 6"
        );
        assert_eq!(
            transaction_container
                .accounts
                .get(&signing_address_1)
                .unwrap()
                .size(),
            1
        );
    }

    #[tokio::test]
    async fn transaction_container_pop_front_account() {
        let mut transaction_container =
            TransactionContainer::new(false, UNSTRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s1_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_4 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            4,
            &signing_key_1,
            [1; 32],
        )));

        // add transactions
        transaction_container.add(ttx_s0_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_1.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_2.clone(), 0).unwrap();
        transaction_container.add(ttx_s1_4.clone(), 0).unwrap();

        // pop from account 1
        assert_eq!(
            transaction_container
                .pop_front_account(&signing_address_0, 1)
                .len(),
            1,
            "one transactions should've been popped"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "empty accounts should be removed"
        );

        // pop from account 2
        assert_eq!(
            transaction_container
                .pop_front_account(&signing_address_1, 1)
                .len(),
            2,
            "two transactions should've been popped"
        );
        assert_eq!(
            transaction_container.accounts.len(),
            1,
            "non empty accounts should not be removed"
        );

        assert_eq!(
            transaction_container.size(),
            1,
            "1 transactions should be remaining from original 4"
        );
    }

    #[tokio::test]
    async fn transaction_container_builder_queue() {
        let mut transaction_container =
            TransactionContainer::new(true, STRICT_SIZE_LIMIT, Duration::from_secs(2));
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.clone().verification_key().address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.clone().verification_key().address_bytes();

        // transactions to add to accounts
        // account, nonce, hash
        let ttx_s0_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_0,
            [1; 32],
        )));
        let ttx_s1_1 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            1,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_2 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            2,
            &signing_key_1,
            [1; 32],
        )));
        let ttx_s1_3 = Arc::new(TimemarkedTransaction::new(get_mock_tx_parameterized(
            3,
            &signing_key_1,
            [1; 32],
        )));

        // add transactions
        transaction_container.add(ttx_s0_1.clone(), 1).unwrap();
        transaction_container.add(ttx_s1_1.clone(), 1).unwrap();
        transaction_container.add(ttx_s1_2.clone(), 1).unwrap();
        transaction_container.add(ttx_s1_3.clone(), 1).unwrap();

        // current nonce getter
        // should return all transactions from signing_key_0 and last two from signing_key_1
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                return Ok(1);
            }
            if address == signing_address_1 {
                return Ok(2);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        // get builder queue
        let mut builder_queue = transaction_container
            .builder_queue(&current_account_nonce_getter)
            .await
            .expect("building builders queue should work");
        assert_eq!(
            builder_queue.len(),
            3,
            "three transactions should've been popped"
        );

        // check that the transactions are in the expected order
        let (first_tx, _) = builder_queue.pop().unwrap();
        assert_eq!(
            first_tx.address(),
            &signing_address_0,
            "expected earliest transaction with lowest nonce difference to be first"
        );
        let (second_tx, _) = builder_queue.pop().unwrap();
        assert_eq!(
            second_tx.address(),
            &signing_address_1,
            "expected lower nonce diff to be second"
        );
        assert_eq!(second_tx.signed_tx().nonce(), 2);
        let (third_tx, _) = builder_queue.pop().unwrap();
        assert_eq!(
            third_tx.address(),
            &signing_address_1,
            "expected highest nonce diff to be last"
        );
        assert_eq!(third_tx.signed_tx().nonce(), 3);

        // ensure transactions not removed
        assert_eq!(
            transaction_container.size(),
            4,
            "no transactions should've been removed"
        );
    }
}
