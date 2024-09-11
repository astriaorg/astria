use std::{
    cmp::Ordering,
    collections::{
        hash_map,
        BTreeMap,
        HashMap,
    },
    fmt,
    future::Future,
    mem,
    sync::Arc,
};

use anyhow::Context;
use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use tokio::time::{
    Duration,
    Instant,
};
use tracing::error;

use super::RemovalReason;

pub(super) type PendingTransactions = TransactionsContainer<PendingTransactionsForAccount>;
pub(super) type ParkedTransactions<const MAX_TX_COUNT: usize> =
    TransactionsContainer<ParkedTransactionsForAccount<MAX_TX_COUNT>>;

/// `TimemarkedTransaction` is a wrapper around a signed transaction used to keep track of when that
/// transaction was first seen in the mempool.
#[derive(Clone, Debug)]
pub(super) struct TimemarkedTransaction {
    signed_tx: Arc<SignedTransaction>,
    tx_hash: [u8; 32],
    time_first_seen: Instant,
    address: [u8; 20],
}

impl TimemarkedTransaction {
    pub(super) fn new(signed_tx: Arc<SignedTransaction>) -> Self {
        Self {
            tx_hash: signed_tx.id().get(),
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

    fn is_expired(&self, now: Instant, ttl: Duration) -> bool {
        now.saturating_duration_since(self.time_first_seen) > ttl
    }

    pub(super) fn nonce(&self) -> u32 {
        self.signed_tx.nonce()
    }

    pub(super) fn address(&self) -> &[u8; 20] {
        &self.address
    }
}

impl fmt::Display for TimemarkedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "tx_hash: {}, address: {}, signer: {}, nonce: {}, chain ID: {}",
            telemetry::display::base64(&self.tx_hash),
            telemetry::display::base64(&self.address),
            self.signed_tx.verification_key(),
            self.signed_tx.nonce(),
            self.signed_tx.chain_id(),
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct TransactionPriority {
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

#[derive(Debug, Eq, PartialEq, Clone)]
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
            InsertionError::AlreadyPresent => {
                write!(f, "transaction already exists in the mempool")
            }
            InsertionError::NonceTooLow => {
                write!(f, "given nonce has already been used previously")
            }
            InsertionError::NonceTaken => write!(f, "given nonce already exists in the mempool"),
            InsertionError::NonceGap => write!(f, "gap in the pending nonce sequence"),
            InsertionError::AccountSizeLimit => write!(
                f,
                "maximum number of pending transactions has been reached for the given account"
            ),
        }
    }
}

/// Transactions for a single account where the sequence of nonces must not have any gaps.
#[derive(Clone, Default, Debug)]
pub(super) struct PendingTransactionsForAccount {
    txs: BTreeMap<u32, TimemarkedTransaction>,
}

impl PendingTransactionsForAccount {
    fn highest_nonce(&self) -> Option<u32> {
        self.txs.last_key_value().map(|(nonce, _)| *nonce)
    }
}

impl TransactionsForAccount for PendingTransactionsForAccount {
    fn txs(&self) -> &BTreeMap<u32, TimemarkedTransaction> {
        &self.txs
    }

    fn txs_mut(&mut self) -> &mut BTreeMap<u32, TimemarkedTransaction> {
        &mut self.txs
    }

    fn is_at_tx_limit(&self) -> bool {
        false
    }

    fn is_sequential_nonce_precondition_met(
        &self,
        ttx: &TimemarkedTransaction,
        current_account_nonce: u32,
    ) -> bool {
        // If the `ttx` nonce is 0, precondition is met iff the current account nonce is also at
        // zero
        let Some(previous_nonce) = ttx.signed_tx.nonce().checked_sub(1) else {
            return current_account_nonce == 0;
        };

        // Precondition is met if the previous nonce is in the existing txs, or if the tx's nonce
        // is equal to the account nonce
        self.txs().contains_key(&previous_nonce) || ttx.signed_tx.nonce() == current_account_nonce
    }
}

/// Transactions for a single account where gaps are allowed in the sequence of nonces, and with an
/// upper bound on the number of transactions.
#[derive(Clone, Default, Debug)]
pub(super) struct ParkedTransactionsForAccount<const MAX_TX_COUNT: usize> {
    txs: BTreeMap<u32, TimemarkedTransaction>,
}

impl<const MAX_TX_COUNT: usize> ParkedTransactionsForAccount<MAX_TX_COUNT> {
    /// Returns contiguous transactions from front of queue starting from target nonce, removing the
    /// transactions in the process.
    ///
    /// Note: this function only operates on the front of the queue. If the target nonce is not at
    /// the front, an error will be logged and nothing will be returned.
    fn pop_front_contiguous(
        &mut self,
        mut target_nonce: u32,
    ) -> impl Iterator<Item = TimemarkedTransaction> {
        let mut split_at = 0;
        for nonce in self.txs.keys() {
            if *nonce == target_nonce {
                let Some(next_target) = target_nonce.checked_add(1) else {
                    // We've got contiguous nonces up to `u32::MAX`; return everything.
                    return mem::take(&mut self.txs).into_values();
                };
                target_nonce = next_target;
                split_at = next_target;
            } else {
                break;
            }
        }

        if split_at == 0 {
            error!(target_nonce, "expected nonce to be present");
        }

        let mut split_off = self.txs.split_off(&split_at);
        // The higher nonces are returned in `split_off`, but we want to keep these in `self.txs`,
        // so swap the two collections.
        mem::swap(&mut split_off, &mut self.txs);
        split_off.into_values()
    }
}

impl<const MAX_TX_COUNT: usize> TransactionsForAccount
    for ParkedTransactionsForAccount<MAX_TX_COUNT>
{
    fn txs(&self) -> &BTreeMap<u32, TimemarkedTransaction> {
        &self.txs
    }

    fn txs_mut(&mut self) -> &mut BTreeMap<u32, TimemarkedTransaction> {
        &mut self.txs
    }

    fn is_at_tx_limit(&self) -> bool {
        self.txs.len() >= MAX_TX_COUNT
    }

    fn is_sequential_nonce_precondition_met(&self, _: &TimemarkedTransaction, _: u32) -> bool {
        true
    }
}

/// `TransactionsForAccount` is a trait for a collection of transactions belonging to a single
/// account.
pub(super) trait TransactionsForAccount: Default {
    fn new() -> Self
    where
        Self: Sized + Default,
    {
        Self::default()
    }

    fn txs(&self) -> &BTreeMap<u32, TimemarkedTransaction>;

    fn txs_mut(&mut self) -> &mut BTreeMap<u32, TimemarkedTransaction>;

    fn is_at_tx_limit(&self) -> bool;

    /// Returns `Ok` if adding `ttx` would not break the nonce precondition, i.e. sequential
    /// nonces with no gaps if in `SequentialNonces` mode.
    fn is_sequential_nonce_precondition_met(
        &self,
        ttx: &TimemarkedTransaction,
        current_account_nonce: u32,
    ) -> bool;

    /// Adds transaction to the container. Note: does NOT allow for nonce replacement.
    /// Will fail if in `SequentialNonces` mode and adding the transaction would create a nonce gap.
    ///
    /// `current_account_nonce` should be the account's nonce in the latest chain state.
    ///
    /// Note: if the account `current_account_nonce` ever decreases, this is a logic error
    /// and could mess up the validity of `SequentialNonces` containers.
    fn add(
        &mut self,
        ttx: TimemarkedTransaction,
        current_account_nonce: u32,
    ) -> Result<(), InsertionError> {
        if self.is_at_tx_limit() {
            return Err(InsertionError::AccountSizeLimit);
        }

        if ttx.nonce() < current_account_nonce {
            return Err(InsertionError::NonceTooLow);
        }

        if let Some(existing_ttx) = self.txs().get(&ttx.signed_tx.nonce()) {
            return Err(if existing_ttx.tx_hash == ttx.tx_hash {
                InsertionError::AlreadyPresent
            } else {
                InsertionError::NonceTaken
            });
        }

        if !self.is_sequential_nonce_precondition_met(&ttx, current_account_nonce) {
            return Err(InsertionError::NonceGap);
        }

        self.txs_mut().insert(ttx.signed_tx.nonce(), ttx);

        Ok(())
    }

    /// Removes transactions with the given nonce and higher.
    ///
    /// Note: the given nonce is expected to be present. If it's absent, an error is logged and no
    /// transactions are removed.
    ///
    /// Returns the hashes of the removed transactions.
    fn remove(&mut self, nonce: u32) -> Vec<[u8; 32]> {
        if !self.txs().contains_key(&nonce) {
            error!(nonce, "transaction with given nonce not found");
            return Vec::new();
        }

        self.txs_mut()
            .split_off(&nonce)
            .values()
            .map(|ttx| ttx.tx_hash)
            .collect()
    }

    /// Returns the transaction with the lowest nonce.
    fn front(&self) -> Option<&TimemarkedTransaction> {
        self.txs().first_key_value().map(|(_, ttx)| ttx)
    }

    /// Removes transactions below the given nonce. Returns the hashes of the removed transactions.
    fn register_latest_account_nonce(
        &mut self,
        current_account_nonce: u32,
    ) -> impl Iterator<Item = [u8; 32]> {
        let mut split_off = self.txs_mut().split_off(&current_account_nonce);
        mem::swap(&mut split_off, self.txs_mut());
        split_off.into_values().map(|ttx| ttx.tx_hash)
    }

    #[cfg(test)]
    fn contains_tx(&self, tx_hash: &[u8; 32]) -> bool {
        self.txs().values().any(|ttx| ttx.tx_hash == *tx_hash)
    }
}

/// `TransactionsContainer` is a container used for managing transactions for multiple accounts.
#[derive(Clone, Debug)]
pub(super) struct TransactionsContainer<T> {
    /// A map of collections of transactions, indexed by the account address.
    txs: HashMap<[u8; 20], T>,
    tx_ttl: Duration,
}

impl<T: TransactionsForAccount> TransactionsContainer<T> {
    pub(super) fn new(tx_ttl: Duration) -> Self {
        TransactionsContainer::<T> {
            txs: HashMap::new(),
            tx_ttl,
        }
    }

    /// Adds the transaction to the container.
    ///
    /// `current_account_nonce` should be the current nonce of the account associated with the
    /// transaction. If this ever decreases, the `TransactionsContainer` containers could become
    /// invalid.
    pub(super) fn add(
        &mut self,
        ttx: TimemarkedTransaction,
        current_account_nonce: u32,
    ) -> Result<(), InsertionError> {
        match self.txs.entry(*ttx.address()) {
            hash_map::Entry::Occupied(entry) => {
                entry.into_mut().add(ttx, current_account_nonce)?;
            }
            hash_map::Entry::Vacant(entry) => {
                let mut txs = T::new();
                txs.add(ttx, current_account_nonce)?;
                entry.insert(txs);
            }
        }
        Ok(())
    }

    /// Removes the given transaction and any transactions with higher nonces for the relevant
    /// account.
    ///
    /// If `signed_tx` existed, returns `Ok` with the hashes of the removed transactions. If
    /// `signed_tx` was not in the collection, it is returned via `Err`.
    pub(super) fn remove(
        &mut self,
        signed_tx: Arc<SignedTransaction>,
    ) -> Result<Vec<[u8; 32]>, Arc<SignedTransaction>> {
        let address = signed_tx.verification_key().address_bytes();

        // Take the collection for this account out of `self` temporarily.
        let Some(mut account_txs) = self.txs.remove(&address) else {
            return Err(signed_tx);
        };

        let removed = account_txs.remove(signed_tx.nonce());

        // Re-add the collection to `self` if it's not empty.
        if !account_txs.txs().is_empty() {
            let _ = self.txs.insert(address, account_txs);
        }

        if removed.is_empty() {
            return Err(signed_tx);
        }

        Ok(removed)
    }

    /// Removes all of the transactions for the given account and returns the hashes of the removed
    /// transactions.
    pub(super) fn clear_account(&mut self, address: &[u8; 20]) -> Vec<[u8; 32]> {
        self.txs
            .remove(address)
            .map(|account_txs| account_txs.txs().values().map(|ttx| ttx.tx_hash).collect())
            .unwrap_or_default()
    }

    /// Cleans all of the accounts in the container. Removes any transactions with stale nonces and
    /// evicts all transactions from accounts whose lowest transaction has expired.
    ///
    /// Returns all transactions that have been removed with the reason why they have been removed.
    pub(super) async fn clean_accounts<F, O>(
        &mut self,
        current_account_nonce_getter: &F,
    ) -> Vec<([u8; 32], RemovalReason)>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        // currently just removes stale nonces and will clear accounts if the
        // transactions are older than the TTL
        let mut accounts_to_remove = Vec::new();
        let mut removed_txs = Vec::new();
        let now = Instant::now();
        for (address, account_txs) in &mut self.txs {
            // check if first tx is older than the TTL, if so, remove all transactions
            if let Some(first_tx) = account_txs.front() {
                if first_tx.is_expired(now, self.tx_ttl) {
                    // first is stale, rest popped for invalidation
                    removed_txs.push((first_tx.tx_hash, RemovalReason::Expired));
                    removed_txs.extend(
                        account_txs
                            .txs()
                            .values()
                            .skip(1)
                            .map(|ttx| (ttx.tx_hash, RemovalReason::LowerNonceInvalidated)),
                    );
                    account_txs.txs_mut().clear();
                } else {
                    // clean to newest nonce
                    let current_account_nonce = match current_account_nonce_getter(*address).await {
                        Ok(nonce) => nonce,
                        Err(error) => {
                            error!(
                                address = %telemetry::display::base64(address),
                                "failed to fetch nonce from state when cleaning accounts: {error:#}"
                            );
                            continue;
                        }
                    };
                    removed_txs.extend(
                        account_txs
                            .register_latest_account_nonce(current_account_nonce)
                            .map(|tx_hash| (tx_hash, RemovalReason::NonceStale)),
                    );
                }
            }

            if account_txs.txs().is_empty() {
                accounts_to_remove.push(*address);
            }
        }

        // remove empty accounts
        for account in accounts_to_remove {
            self.txs.remove(&account);
        }

        removed_txs
    }

    /// Returns the number of transactions in the container.
    pub(super) fn len(&self) -> usize {
        self.txs
            .values()
            .map(|account_txs| account_txs.txs().len())
            .sum()
    }

    #[cfg(test)]
    fn contains_tx(&self, tx_hash: &[u8; 32]) -> bool {
        self.txs
            .values()
            .any(|account_txs| account_txs.contains_tx(tx_hash))
    }
}

impl TransactionsContainer<PendingTransactionsForAccount> {
    /// Returns the highest nonce for an account.
    pub(super) fn pending_nonce(&self, address: [u8; 20]) -> Option<u32> {
        self.txs
            .get(&address)
            .and_then(PendingTransactionsForAccount::highest_nonce)
    }

    /// Returns a copy of transactions and their hashes sorted by nonce difference and then time
    /// first seen.
    pub(super) async fn builder_queue<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<Vec<([u8; 32], Arc<SignedTransaction>)>>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        // Used to hold the values in Vec for sorting.
        struct QueueEntry {
            tx: Arc<SignedTransaction>,
            tx_hash: [u8; 32],
            priority: TransactionPriority,
        }

        let mut queue = Vec::with_capacity(self.len());
        // Add all transactions to the queue.
        for (address, account_txs) in &self.txs {
            let current_account_nonce = current_account_nonce_getter(*address)
                .await
                .context("failed to fetch account nonce for builder queue")?;
            for ttx in account_txs.txs.values() {
                let priority = match ttx.priority(current_account_nonce) {
                    Ok(priority) => priority,
                    Err(error) => {
                        // mempool could be off due to node connectivity issues
                        error!(
                            tx_hash = %telemetry::display::base64(&ttx.tx_hash),
                            "failed to add pending tx to builder queue: {error:#}"
                        );
                        continue;
                    }
                };
                queue.push(QueueEntry {
                    tx: ttx.signed_tx.clone(),
                    tx_hash: ttx.tx_hash,
                    priority,
                });
            }
        }

        // Sort the queue and return the relevant data. Note that the sorted queue will be ordered
        // from lowest to highest priority, so we need to reverse the order before returning.
        queue.sort_unstable_by_key(|entry| entry.priority);
        Ok(queue
            .into_iter()
            .rev()
            .map(|entry| (entry.tx_hash, entry.tx))
            .collect())
    }
}

impl<const MAX_TX_COUNT: usize> TransactionsContainer<ParkedTransactionsForAccount<MAX_TX_COUNT>> {
    /// Removes and returns the transactions from the front of an account, similar to
    /// `find_promotables`. Useful for when needing to promote transactions from a specific
    /// account instead of all accounts.
    pub(super) fn pop_front_account(
        &mut self,
        account: &[u8; 20],
        target_nonce: u32,
    ) -> Vec<TimemarkedTransaction> {
        // Take the collection for this account out of `self` temporarily.
        let Some(mut account_txs) = self.txs.remove(account) else {
            return Vec::new();
        };

        let removed = account_txs.pop_front_contiguous(target_nonce);

        // Re-add the collection to `self` if it's not empty.
        if !account_txs.txs().is_empty() {
            let _ = self.txs.insert(*account, account_txs);
        }
        removed.collect()
    }

    /// Removes and returns transactions along with their account's current nonce that are lower
    /// than or equal to that nonce. This is helpful when needing to promote transactions from
    /// parked to pending during mempool maintenance.
    pub(super) async fn find_promotables<F, O>(
        &mut self,
        current_account_nonce_getter: &F,
    ) -> Vec<(TimemarkedTransaction, u32)>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        let mut accounts_to_remove = Vec::new();
        let mut promoted_txs = Vec::new();

        for (address, account_txs) in &mut self.txs {
            let current_account_nonce = match current_account_nonce_getter(*address).await {
                Ok(nonce) => nonce,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(address),
                        "failed to fetch nonce from state when finding promotables: {error:#}"
                    );
                    continue;
                }
            };

            // find transactions that can be promoted
            // note: can use current account nonce as target because this logic
            // is only handling the case where transactions we didn't have in our
            // local mempool were ran that would enable the parked transactions to
            // be valid
            promoted_txs.extend(
                account_txs
                    .pop_front_contiguous(current_account_nonce)
                    .map(|ttx| (ttx, current_account_nonce)),
            );

            if account_txs.txs.is_empty() {
                accounts_to_remove.push(*address);
            }
        }

        // remove empty accounts
        for account in accounts_to_remove {
            self.txs.remove(&account);
        }

        promoted_txs
    }
}

#[cfg(test)]
mod test {
    use astria_core::crypto::SigningKey;

    use super::*;
    use crate::app::test_utils::mock_tx;

    const MAX_PARKED_TXS_PER_ACCOUNT: usize = 15;
    const TX_TTL: Duration = Duration::from_secs(2);

    fn mock_ttx(nonce: u32, signer: &SigningKey) -> TimemarkedTransaction {
        TimemarkedTransaction::new(mock_tx(nonce, signer, "test"))
    }

    #[test]
    fn transaction_priority_should_error_if_invalid() {
        let ttx = TimemarkedTransaction::new(mock_tx(0, &[1; 32].into(), "test"));
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
    fn parked_transactions_for_account_add() {
        let mut parked_txs = ParkedTransactionsForAccount::<MAX_PARKED_TXS_PER_ACCOUNT>::new();

        // transactions to add
        let ttx_1 = mock_ttx(1, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());
        let ttx_5 = mock_ttx(5, &[1; 32].into());

        let current_account_nonce = 2;
        parked_txs
            .add(ttx_3.clone(), current_account_nonce)
            .unwrap();
        assert!(parked_txs.contains_tx(&ttx_3.tx_hash));
        assert_eq!(
            parked_txs.add(ttx_3, current_account_nonce).unwrap_err(),
            InsertionError::AlreadyPresent
        );

        // add gapped transaction
        parked_txs.add(ttx_5, current_account_nonce).unwrap();

        // fail adding too low nonce
        assert_eq!(
            parked_txs.add(ttx_1, current_account_nonce).unwrap_err(),
            InsertionError::NonceTooLow
        );
    }

    #[test]
    fn parked_transactions_for_account_size_limit() {
        let mut parked_txs = ParkedTransactionsForAccount::<2>::new();

        // transactions to add
        let ttx_1 = mock_ttx(1, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());
        let ttx_5 = mock_ttx(5, &[1; 32].into());

        let current_account_nonce = 0;
        parked_txs
            .add(ttx_3.clone(), current_account_nonce)
            .unwrap();
        parked_txs.add(ttx_5, current_account_nonce).unwrap();

        // fail with size limit hit
        assert_eq!(
            parked_txs.add(ttx_1, current_account_nonce).unwrap_err(),
            InsertionError::AccountSizeLimit
        );
    }

    #[test]
    fn pending_transactions_for_account_add() {
        let mut pending_txs = PendingTransactionsForAccount::new();

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_1 = mock_ttx(1, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());

        let current_account_nonce = 1;

        // too low nonces not added
        assert_eq!(
            pending_txs.add(ttx_0, current_account_nonce).unwrap_err(),
            InsertionError::NonceTooLow
        );
        assert!(pending_txs.txs().is_empty());

        // too high nonces with empty container not added
        assert_eq!(
            pending_txs
                .add(ttx_2.clone(), current_account_nonce)
                .unwrap_err(),
            InsertionError::NonceGap
        );
        assert!(pending_txs.txs().is_empty());

        // add ok
        pending_txs
            .add(ttx_1.clone(), current_account_nonce)
            .unwrap();
        assert_eq!(
            pending_txs.add(ttx_1, current_account_nonce).unwrap_err(),
            InsertionError::AlreadyPresent
        );

        // gapped transaction not allowed
        assert_eq!(
            pending_txs.add(ttx_3, current_account_nonce).unwrap_err(),
            InsertionError::NonceGap
        );

        // can add consecutive
        pending_txs.add(ttx_2, current_account_nonce).unwrap();
    }

    #[test]
    fn transactions_for_account_remove() {
        let mut account_txs = PendingTransactionsForAccount::new();

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_1 = mock_ttx(1, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());

        account_txs.add(ttx_0.clone(), 0).unwrap();
        account_txs.add(ttx_1.clone(), 0).unwrap();
        account_txs.add(ttx_2.clone(), 0).unwrap();
        account_txs.add(ttx_3.clone(), 0).unwrap();

        // remove from end will only remove end
        assert_eq!(
            account_txs.remove(3),
            vec![ttx_3.tx_hash],
            "only one transaction should've been removed"
        );
        assert_eq!(account_txs.txs().len(), 3);

        // remove same again return nothing
        assert_eq!(
            account_txs.remove(3).len(),
            0,
            "no transaction should be removed"
        );
        assert_eq!(account_txs.txs().len(), 3);

        // remove from start will remove all
        assert_eq!(
            account_txs.remove(0),
            vec![ttx_0.tx_hash, ttx_1.tx_hash, ttx_2.tx_hash,],
            "three transactions should've been removed"
        );
        assert!(account_txs.txs().is_empty());
    }

    #[test]
    fn parked_transactions_for_account_pop_front_contiguous() {
        let mut parked_txs = ParkedTransactionsForAccount::<MAX_PARKED_TXS_PER_ACCOUNT>::new();

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());
        let ttx_4 = mock_ttx(4, &[1; 32].into());

        parked_txs.add(ttx_0.clone(), 0).unwrap();
        parked_txs.add(ttx_2.clone(), 0).unwrap();
        parked_txs.add(ttx_3.clone(), 0).unwrap();
        parked_txs.add(ttx_4.clone(), 0).unwrap();

        // lowest nonce not target nonce is noop
        assert_eq!(
            parked_txs.pop_front_contiguous(2).count(),
            0,
            "no transaction should've been removed"
        );
        assert_eq!(parked_txs.txs().len(), 4);

        // will remove single value
        assert_eq!(
            parked_txs
                .pop_front_contiguous(0)
                .map(|ttx| ttx.tx_hash)
                .collect::<Vec<_>>(),
            vec![ttx_0.tx_hash],
            "single transaction should've been returned"
        );
        assert_eq!(parked_txs.txs().len(), 3);

        // will remove multiple values
        assert_eq!(
            parked_txs
                .pop_front_contiguous(2)
                .map(|ttx| ttx.tx_hash)
                .collect::<Vec<_>>(),
            vec![ttx_2.tx_hash, ttx_3.tx_hash, ttx_4.tx_hash],
            "multiple transaction should've been returned"
        );
        assert!(parked_txs.txs().is_empty());
    }

    #[test]
    fn pending_transactions_for_account_highest_nonce() {
        let mut pending_txs = PendingTransactionsForAccount::new();

        // no transactions ok
        assert!(
            pending_txs.highest_nonce().is_none(),
            "no transactions will return None"
        );

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_1 = mock_ttx(1, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());

        pending_txs.add(ttx_0, 0).unwrap();
        pending_txs.add(ttx_1, 0).unwrap();
        pending_txs.add(ttx_2, 0).unwrap();

        // will return last transaction
        assert_eq!(
            pending_txs.highest_nonce(),
            Some(2),
            "highest nonce should be returned"
        );
    }

    #[test]
    fn transactions_for_account_front() {
        let mut parked_txs = ParkedTransactionsForAccount::<MAX_PARKED_TXS_PER_ACCOUNT>::new();

        // no transactions ok
        assert!(
            parked_txs.front().is_none(),
            "no transactions will return None"
        );

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());

        parked_txs.add(ttx_0.clone(), 0).unwrap();
        parked_txs.add(ttx_2, 0).unwrap();

        // will return first transaction
        assert_eq!(
            parked_txs.front().unwrap().tx_hash,
            ttx_0.tx_hash,
            "lowest transaction should be returned"
        );
    }

    #[test]
    fn transactions_for_account_register_latest_account_nonce() {
        let mut parked_txs = ParkedTransactionsForAccount::<MAX_PARKED_TXS_PER_ACCOUNT>::new();

        // transactions to add
        let ttx_0 = mock_ttx(0, &[1; 32].into());
        let ttx_2 = mock_ttx(2, &[1; 32].into());
        let ttx_3 = mock_ttx(3, &[1; 32].into());
        let ttx_4 = mock_ttx(4, &[1; 32].into());

        parked_txs.add(ttx_0.clone(), 0).unwrap();
        parked_txs.add(ttx_2.clone(), 0).unwrap();
        parked_txs.add(ttx_3.clone(), 0).unwrap();
        parked_txs.add(ttx_4.clone(), 0).unwrap();

        // matching nonce will not be removed
        assert_eq!(
            parked_txs.register_latest_account_nonce(0).count(),
            0,
            "no transaction should've been removed"
        );
        assert_eq!(parked_txs.txs().len(), 4);

        // fast forwarding to non existing middle nonce ok
        assert_eq!(
            parked_txs
                .register_latest_account_nonce(1)
                .collect::<Vec<_>>(),
            vec![ttx_0.tx_hash],
            "ttx_0 should've been removed"
        );
        assert_eq!(parked_txs.txs().len(), 3);

        // fast forwarding to existing nonce ok
        assert_eq!(
            parked_txs
                .register_latest_account_nonce(3)
                .collect::<Vec<_>>(),
            vec![ttx_2.tx_hash],
            "one transaction should've been removed"
        );
        assert_eq!(parked_txs.txs().len(), 2);

        // fast forwarding to much higher nonce ok
        assert_eq!(
            parked_txs
                .register_latest_account_nonce(10)
                .collect::<Vec<_>>(),
            vec![ttx_3.tx_hash, ttx_4.tx_hash],
            "two transactions should've been removed"
        );
        assert!(parked_txs.txs().is_empty());
    }

    #[test]
    fn transactions_container_add() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add to accounts
        let ttx_s0_0_0 = mock_ttx(0, &signing_key_0);
        // Same nonce and signer as `ttx_s0_0_0`, but different rollup name, hence different tx.
        let ttx_s0_0_1 = TimemarkedTransaction::new(mock_tx(0, &signing_key_0, "other"));
        let ttx_s0_2_0 = mock_ttx(2, &signing_key_0);
        let ttx_s1_0_0 = mock_ttx(0, &signing_key_1);

        // transactions to add for account 1

        // initially no accounts should exist
        assert!(
            pending_txs.txs.is_empty(),
            "no accounts should exist at first"
        );

        // adding too low nonce shouldn't create account
        assert_eq!(
            pending_txs.add(ttx_s0_0_0.clone(), 1).unwrap_err(),
            InsertionError::NonceTooLow,
            "shouldn't be able to add nonce too low transaction"
        );
        assert!(
            pending_txs.txs.is_empty(),
            "failed adds to new accounts shouldn't create account"
        );

        // add one transaction
        pending_txs.add(ttx_s0_0_0.clone(), 0).unwrap();
        assert_eq!(pending_txs.txs.len(), 1, "one account should exist");

        // re-adding transaction should fail
        assert_eq!(
            pending_txs.add(ttx_s0_0_0, 0).unwrap_err(),
            InsertionError::AlreadyPresent,
            "re-adding same transaction should fail"
        );

        // nonce replacement fails
        assert_eq!(
            pending_txs.add(ttx_s0_0_1, 0).unwrap_err(),
            InsertionError::NonceTaken,
            "nonce replacement not supported"
        );

        // nonce gaps not supported
        assert_eq!(
            pending_txs.add(ttx_s0_2_0, 0).unwrap_err(),
            InsertionError::NonceGap,
            "gapped nonces in pending transactions not allowed"
        );

        // add transactions for account 2
        pending_txs.add(ttx_s1_0_0, 0).unwrap();

        // check internal structures
        assert_eq!(pending_txs.txs.len(), 2, "two accounts should exist");
        assert_eq!(
            pending_txs.txs.get(&signing_address_0).unwrap().txs().len(),
            1,
            "one transaction should be in the original account"
        );
        assert_eq!(
            pending_txs.txs.get(&signing_address_1).unwrap().txs().len(),
            1,
            "one transaction should be in the second account"
        );
        assert_eq!(
            pending_txs.len(),
            2,
            "should only have two transactions tracked"
        );
    }

    #[test]
    fn transactions_container_remove() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_key_1 = SigningKey::from([2; 32]);

        // transactions to add to accounts
        let ttx_s0_0 = mock_ttx(0, &signing_key_0);
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s1_0 = mock_ttx(0, &signing_key_1);
        let ttx_s1_1 = mock_ttx(1, &signing_key_1);

        // remove on empty returns the tx in Err variant.
        assert!(
            pending_txs.remove(ttx_s0_0.signed_tx.clone()).is_err(),
            "zero transactions should be removed from non existing accounts"
        );

        // add transactions
        pending_txs.add(ttx_s0_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s0_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_1.clone(), 0).unwrap();

        // remove should remove tx and higher
        assert_eq!(
            pending_txs.remove(ttx_s0_0.signed_tx.clone()).unwrap(),
            vec![ttx_s0_0.tx_hash, ttx_s0_1.tx_hash],
            "rest of transactions for account should be removed when targeting bottom nonce"
        );
        assert_eq!(pending_txs.txs.len(), 1, "empty account should be removed");
        assert_eq!(
            pending_txs.len(),
            2,
            "should only have two transactions tracked"
        );
        assert!(
            pending_txs.contains_tx(&ttx_s1_0.tx_hash),
            "other account should be untouched"
        );
        assert!(
            pending_txs.contains_tx(&ttx_s1_1.tx_hash),
            "other account should be untouched"
        );
    }

    #[test]
    fn transactions_container_clear_account() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);

        // transactions to add to accounts
        let ttx_s0_0 = mock_ttx(0, &signing_key_0);
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s1_0 = mock_ttx(0, &signing_key_1);

        // clear all on empty returns zero
        assert!(
            pending_txs.clear_account(&signing_address_0).is_empty(),
            "zero transactions should be removed from clearing non existing accounts"
        );

        // add transactions
        pending_txs.add(ttx_s0_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s0_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_0.clone(), 0).unwrap();

        // clear should return all transactions
        assert_eq!(
            pending_txs.clear_account(&signing_address_0),
            vec![ttx_s0_0.tx_hash, ttx_s0_1.tx_hash],
            "all transactions should be returned from clearing account"
        );

        assert_eq!(pending_txs.txs.len(), 1, "empty account should be removed");
        assert_eq!(
            pending_txs.len(),
            1,
            "should only have one transaction tracked"
        );
        assert!(
            pending_txs.contains_tx(&ttx_s1_0.tx_hash),
            "other account should be untouched"
        );
    }

    #[tokio::test]
    async fn transactions_container_clean_accounts() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();
        let signing_key_2 = SigningKey::from([3; 32]);
        let signing_address_2 = signing_key_2.address_bytes();

        // transactions to add to accounts
        let ttx_s0_0 = mock_ttx(0, &signing_key_0);
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s0_2 = mock_ttx(2, &signing_key_0);
        let ttx_s1_0 = mock_ttx(0, &signing_key_1);
        let ttx_s1_1 = mock_ttx(1, &signing_key_1);
        let ttx_s1_2 = mock_ttx(2, &signing_key_1);
        let ttx_s2_0 = mock_ttx(0, &signing_key_2);
        let ttx_s2_1 = mock_ttx(1, &signing_key_2);
        let ttx_s2_2 = mock_ttx(2, &signing_key_2);

        // add transactions
        pending_txs.add(ttx_s0_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s0_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s0_2.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_2.clone(), 0).unwrap();
        pending_txs.add(ttx_s2_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s2_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s2_2.clone(), 0).unwrap();

        // current nonce getter
        // should pop none from signing_address_0, one from signing_address_1, and all from
        // signing_address_2
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                Ok(0)
            } else if address == signing_address_1 {
                Ok(1)
            } else if address == signing_address_2 {
                Ok(4)
            } else {
                Err(anyhow::anyhow!("invalid address"))
            }
        };

        let removed_txs = pending_txs
            .clean_accounts(&current_account_nonce_getter)
            .await;

        assert_eq!(
            removed_txs.len(),
            4,
            "four transactions should've been popped"
        );
        assert_eq!(pending_txs.txs.len(), 2, "empty accounts should be removed");
        assert_eq!(
            pending_txs.len(),
            5,
            "5 transactions should be remaining from original 9"
        );
        assert!(pending_txs.contains_tx(&ttx_s0_0.tx_hash));
        assert!(pending_txs.contains_tx(&ttx_s0_1.tx_hash));
        assert!(pending_txs.contains_tx(&ttx_s0_2.tx_hash));
        assert!(pending_txs.contains_tx(&ttx_s1_1.tx_hash));
        assert!(pending_txs.contains_tx(&ttx_s1_2.tx_hash));

        assert_eq!(
            pending_txs.txs.get(&signing_address_0).unwrap().txs().len(),
            3
        );
        assert_eq!(
            pending_txs.txs.get(&signing_address_1).unwrap().txs().len(),
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
    async fn transactions_container_clean_accounts_expired_transactions() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add to accounts
        let ttx_s0_0 = mock_ttx(0, &signing_key_0);

        // pass time to make first transaction stale
        tokio::time::advance(TX_TTL.saturating_add(Duration::from_nanos(1))).await;

        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s1_0 = mock_ttx(0, &signing_key_1);

        // add transactions
        pending_txs.add(ttx_s0_0.clone(), 0).unwrap();
        pending_txs.add(ttx_s0_1.clone(), 0).unwrap();
        pending_txs.add(ttx_s1_0.clone(), 0).unwrap();

        // current nonce getter
        // all nonces should be valid
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 || address == signing_address_1 {
                return Ok(0);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        let removed_txs = pending_txs
            .clean_accounts(&current_account_nonce_getter)
            .await;

        assert_eq!(
            removed_txs.len(),
            2,
            "two transactions should've been popped"
        );
        assert_eq!(pending_txs.txs.len(), 1, "empty accounts should be removed");
        assert_eq!(
            pending_txs.len(),
            1,
            "1 transaction should be remaining from original 3"
        );
        assert!(
            pending_txs.contains_tx(&ttx_s1_0.tx_hash),
            "not expired account should be untouched"
        );

        // check removal reasons
        assert_eq!(
            removed_txs[0],
            (ttx_s0_0.tx_hash, RemovalReason::Expired),
            "first should be first pushed tx with removal reason as expired"
        );
        assert_eq!(
            removed_txs[1],
            (ttx_s0_1.tx_hash, RemovalReason::LowerNonceInvalidated),
            "second should be second added tx with removal reason as lower nonce invalidation"
        );
    }

    #[test]
    fn pending_transactions_pending_nonce() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();

        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add for account 0
        let ttx_s0_0 = mock_ttx(0, &signing_key_0);
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);

        pending_txs.add(ttx_s0_0, 0).unwrap();
        pending_txs.add(ttx_s0_1, 0).unwrap();

        // empty account returns zero
        assert!(
            pending_txs.pending_nonce(signing_address_1).is_none(),
            "empty account should return None"
        );

        // non empty account returns highest nonce
        assert_eq!(
            pending_txs.pending_nonce(signing_address_0),
            Some(1),
            "should return highest nonce"
        );
    }

    #[tokio::test]
    async fn pending_transactions_builder_queue() {
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add to accounts
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s1_1 = mock_ttx(1, &signing_key_1);
        let ttx_s1_2 = mock_ttx(2, &signing_key_1);
        let ttx_s1_3 = mock_ttx(3, &signing_key_1);

        // add transactions
        pending_txs.add(ttx_s0_1.clone(), 1).unwrap();
        pending_txs.add(ttx_s1_1.clone(), 1).unwrap();
        pending_txs.add(ttx_s1_2.clone(), 1).unwrap();
        pending_txs.add(ttx_s1_3.clone(), 1).unwrap();

        // current nonce getter
        // should return all transactions from signing_key_0 and last two from signing_key_1
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 {
                Ok(1)
            } else if address == signing_address_1 {
                Ok(2)
            } else {
                Err(anyhow::anyhow!("invalid address"))
            }
        };

        // get builder queue
        let builder_queue = pending_txs
            .builder_queue(&current_account_nonce_getter)
            .await
            .expect("building builders queue should work");
        assert_eq!(
            builder_queue.len(),
            3,
            "three transactions should've been popped"
        );

        // check that the transactions are in the expected order
        let (first_tx_hash, _) = builder_queue[0];
        assert_eq!(
            first_tx_hash, ttx_s0_1.tx_hash,
            "expected earliest transaction with lowest nonce difference (0) to be first"
        );
        let (second_tx_hash, _) = builder_queue[1];
        assert_eq!(
            second_tx_hash, ttx_s1_2.tx_hash,
            "expected other low nonce diff (0) to be second"
        );
        let (third_tx_hash, _) = builder_queue[2];
        assert_eq!(
            third_tx_hash, ttx_s1_3.tx_hash,
            "expected highest nonce diff to be last"
        );

        // ensure transactions not removed
        assert_eq!(
            pending_txs.len(),
            4,
            "no transactions should've been removed"
        );
    }

    #[tokio::test]
    async fn parked_transactions_pop_front_account() {
        let mut parked_txs = ParkedTransactions::<MAX_PARKED_TXS_PER_ACCOUNT>::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add to accounts
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s1_1 = mock_ttx(1, &signing_key_1);
        let ttx_s1_2 = mock_ttx(2, &signing_key_1);
        let ttx_s1_4 = mock_ttx(4, &signing_key_1);

        // add transactions
        parked_txs.add(ttx_s0_1.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_1.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_2.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_4.clone(), 0).unwrap();

        // pop from account 1
        assert_eq!(
            parked_txs.pop_front_account(&signing_address_0, 1).len(),
            1,
            "one transactions should've been popped"
        );
        assert_eq!(parked_txs.txs.len(), 1, "empty accounts should be removed");

        // pop from account 2
        assert_eq!(
            parked_txs.pop_front_account(&signing_address_1, 1).len(),
            2,
            "two transactions should've been popped"
        );
        assert_eq!(
            parked_txs.txs.len(),
            1,
            "non empty accounts should not be removed"
        );

        assert_eq!(
            parked_txs.len(),
            1,
            "1 transactions should be remaining from original 4"
        );
        assert!(parked_txs.contains_tx(&ttx_s1_4.tx_hash));
    }

    #[tokio::test]
    async fn parked_transactions_find_promotables() {
        let mut parked_txs = ParkedTransactions::<MAX_PARKED_TXS_PER_ACCOUNT>::new(TX_TTL);
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_address_0 = signing_key_0.address_bytes();
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_address_1 = signing_key_1.address_bytes();

        // transactions to add to accounts
        let ttx_s0_1 = mock_ttx(1, &signing_key_0);
        let ttx_s0_2 = mock_ttx(2, &signing_key_0);
        let ttx_s0_3 = mock_ttx(3, &signing_key_0);
        let ttx_s1_1 = mock_ttx(1, &signing_key_1);
        let ttx_s1_2 = mock_ttx(2, &signing_key_1);
        let ttx_s1_4 = mock_ttx(4, &signing_key_1);

        // add transactions
        parked_txs.add(ttx_s0_1.clone(), 0).unwrap();
        parked_txs.add(ttx_s0_2.clone(), 0).unwrap();
        parked_txs.add(ttx_s0_3.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_1.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_2.clone(), 0).unwrap();
        parked_txs.add(ttx_s1_4.clone(), 0).unwrap();

        // current nonce getter
        // should pop all from signing_address_0 and two from signing_address_1
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address_0 || address == signing_address_1 {
                return Ok(1);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        assert_eq!(
            parked_txs
                .find_promotables(&current_account_nonce_getter)
                .await
                .len(),
            5,
            "five transactions should've been popped"
        );
        assert_eq!(parked_txs.txs.len(), 1, "empty accounts should be removed");
        assert_eq!(
            parked_txs.len(),
            1,
            "1 transactions should be remaining from original 6"
        );
        assert!(parked_txs.contains_tx(&ttx_s1_4.tx_hash));
    }
}
