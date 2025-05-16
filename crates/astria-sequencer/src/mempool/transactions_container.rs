use std::{
    cmp::Ordering,
    collections::{
        hash_map,
        BTreeMap,
        HashMap,
        HashSet,
    },
    fmt,
    mem,
    sync::Arc,
};

use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
    },
    protocol::transaction::v1::action::group::Group,
};
use astria_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};
use tokio::time::{
    Duration,
    Instant,
};
use tracing::{
    error,
    instrument,
};

use super::RemovalReason;
use crate::{
    accounts,
    accounts::AddressBytes as _,
    checked_transaction::CheckedTransaction,
};

/// `TimemarkedTransaction` is a wrapper around a checked transaction used to keep track of when
/// that transaction was first seen in the mempool and its total cost to execute.
#[derive(Clone, Debug)]
pub(super) struct TimemarkedTransaction {
    checked_tx: Arc<CheckedTransaction>,
    time_first_seen: Instant,
    costs: HashMap<IbcPrefixed, u128>,
}

impl TimemarkedTransaction {
    pub(super) fn new(
        checked_tx: Arc<CheckedTransaction>,
        costs: HashMap<IbcPrefixed, u128>,
    ) -> Self {
        Self {
            checked_tx,
            time_first_seen: Instant::now(),
            costs,
        }
    }

    fn priority(&self, current_account_nonce: u32) -> Result<TransactionPriority> {
        let Some(nonce_diff) = self.checked_tx.nonce().checked_sub(current_account_nonce) else {
            return Err(eyre!(
                "transaction nonce {} is less than current account nonce {current_account_nonce}",
                self.checked_tx.nonce()
            ));
        };

        Ok(TransactionPriority {
            nonce_diff,
            time_first_seen: self.time_first_seen,
            group: self.checked_tx.group(),
        })
    }

    pub(super) fn deduct_costs(
        &self,
        available_balances: &mut HashMap<IbcPrefixed, u128>,
    ) -> Result<()> {
        self.costs.iter().try_for_each(|(denom, cost)| {
            if *cost == 0 {
                return Ok(());
            }
            let Some(current_balance) = available_balances.get_mut(denom) else {
                return Err(eyre!("account missing balance for {denom}"));
            };
            let Some(new_balance) = current_balance.checked_sub(*cost) else {
                return Err(eyre!("cost greater than account's balance for {denom}"));
            };
            *current_balance = new_balance;
            Ok(())
        })
    }

    async fn recalculate_costs<S: accounts::StateReadExt>(&mut self, state: &S) -> Result<()> {
        self.costs = self
            .checked_tx
            .total_costs(state)
            .await
            .wrap_err("failed to recalculate tx costs")?;
        Ok(())
    }

    fn is_expired(&self, now: Instant, ttl: Duration) -> bool {
        now.saturating_duration_since(self.time_first_seen) > ttl
    }

    pub(super) fn id(&self) -> &TransactionId {
        self.checked_tx.id()
    }

    pub(super) fn nonce(&self) -> u32 {
        self.checked_tx.nonce()
    }

    pub(super) fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        self.checked_tx.address_bytes()
    }
}

impl fmt::Display for TimemarkedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "tx_id: {}, address: {}, signer: {}, nonce: {}, chain ID: {}, group: {}",
            self.id(),
            telemetry::display::base64(self.address_bytes()),
            self.checked_tx.verification_key(),
            self.checked_tx.nonce(),
            self.checked_tx.chain_id(),
            self.checked_tx.group(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TransactionPriority {
    nonce_diff: u32,
    time_first_seen: Instant,
    group: Group,
}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // first ordered by group
        let group = self.group.cmp(&other.group);
        if group != Ordering::Equal {
            return group;
        }

        // then by nonce difference where lower nonce diff means higher priority
        let nonce_diff = self.nonce_diff.cmp(&other.nonce_diff).reverse();
        if nonce_diff != Ordering::Equal {
            return nonce_diff;
        }

        // then by timestamp if nonce and group are equal
        self.time_first_seen.cmp(&other.time_first_seen).reverse()
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
    AccountBalanceTooLow,
    ParkedSizeLimit,
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
            InsertionError::AccountBalanceTooLow => {
                write!(f, "account does not have enough balance to cover costs")
            }
            InsertionError::ParkedSizeLimit => {
                write!(f, "parked container size limit reached")
            }
        }
    }
}

impl From<InsertionError> for tonic::Status {
    fn from(err: InsertionError) -> Self {
        match err {
            InsertionError::AlreadyPresent | InsertionError::NonceTaken => {
                tonic::Status::already_exists(err.to_string())
            }
            InsertionError::NonceTooLow | InsertionError::NonceGap => {
                tonic::Status::invalid_argument(err.to_string())
            }
            InsertionError::AccountSizeLimit | InsertionError::ParkedSizeLimit => {
                tonic::Status::resource_exhausted(err.to_string())
            }
            InsertionError::AccountBalanceTooLow => {
                tonic::Status::failed_precondition(err.to_string())
            }
        }
    }
}

/// Transactions for a single account where the sequence of nonces must not have any gaps.
/// Contains logic to restrict total cost of contained transactions to inputted balances.
#[derive(Clone, Default, Debug)]
pub(super) struct PendingTransactionsForAccount {
    txs: BTreeMap<u32, TimemarkedTransaction>,
}

impl PendingTransactionsForAccount {
    fn pending_account_nonce(&self) -> Option<u32> {
        // The account nonce is the number of transactions executed on the
        // account. Tx nonce must be equal to this number at execution, thus
        // account nonce is always one higher than the last executed tx nonce.
        // The pending account nonce is what account nonce will be after all txs
        // have executed. This is the highest nonce in the pending txs + 1.
        self.txs
            .last_key_value()
            .map(|(nonce, _)| nonce.saturating_add(1))
    }

    fn current_account_nonce(&self) -> Option<u32> {
        self.txs.first_key_value().map(|(nonce, _)| *nonce)
    }

    /// Removes and returns transactions that exceed the balances in `available_balances`.
    fn find_demotables(
        &mut self,
        mut available_balances: HashMap<IbcPrefixed, u128>,
    ) -> Vec<TimemarkedTransaction> {
        let mut split_at = 0;

        for (nonce, tx) in &self.txs {
            // ensure we have enough balance to cover inclusion
            if tx.deduct_costs(&mut available_balances).is_err() {
                break;
            }

            split_at = nonce.saturating_add(1);
        }

        // return all keys higher than split target
        self.txs.split_off(&split_at).into_values().collect()
    }

    /// Returns remaining balances after accounting for costs of contained transactions.
    ///
    /// Note: assumes that the balances in `account_balances` are large enough
    /// to cover costs for contained transactions. Will log an error if this is not true
    /// but will not fail.
    fn subtract_contained_costs(&self, account_balances: &mut HashMap<IbcPrefixed, u128>) {
        // deduct costs from current account balances
        self.txs.values().for_each(|tx| {
            tx.costs.iter().for_each(|(denom, cost)| {
                if *cost == 0 {
                    return;
                }
                let Some(current_balance) = account_balances.get_mut(denom) else {
                    error!("pending transactions has cost not in account balances");
                    return;
                };
                let new_balance = current_balance.checked_sub(*cost).unwrap_or_else(|| {
                    error!("pending transaction cost greater than available account balance");
                    0
                });
                *current_balance = new_balance;
            });
        });
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
        let Some(previous_nonce) = ttx.nonce().checked_sub(1) else {
            return current_account_nonce == 0;
        };

        // Precondition is met if the previous nonce is in the existing txs, or if the tx's nonce
        // is equal to the account nonce
        self.txs().contains_key(&previous_nonce) || ttx.nonce() == current_account_nonce
    }

    fn has_balance_to_cover(
        &self,
        ttx: &TimemarkedTransaction,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
    ) -> bool {
        let mut current_account_balances = current_account_balances.clone();
        self.txs
            .values()
            .chain(std::iter::once(ttx))
            .try_for_each(|ttx| ttx.deduct_costs(&mut current_account_balances))
            .is_ok()
    }
}

/// Transactions for a single account where gaps are allowed in the sequence of nonces, and with an
/// upper bound on the number of transactions.
#[derive(Clone, Default, Debug)]
pub(super) struct ParkedTransactionsForAccount<const MAX_TX_COUNT: usize> {
    txs: BTreeMap<u32, TimemarkedTransaction>,
}

impl<const MAX_TX_COUNT: usize> ParkedTransactionsForAccount<MAX_TX_COUNT> {
    /// Returns contiguous transactions from front of queue starting from `target_nonce`, removing
    /// the transactions in the process. Will only return transactions if their cost is covered
    /// by the `available_balances`.
    ///
    /// `target_nonce` should be the next nonce that the pending queue could add.
    ///
    /// Note: this function only operates on the front of the queue. If the target nonce is not at
    /// the front, nothing will be returned.
    fn find_promotables(
        &mut self,
        mut target_nonce: u32,
        mut available_balances: HashMap<IbcPrefixed, u128>,
    ) -> impl Iterator<Item = TimemarkedTransaction> {
        let mut split_at: u32 = 0;
        for (nonce, ttx) in &self.txs {
            if *nonce != target_nonce || ttx.deduct_costs(&mut available_balances).is_err() {
                break;
            }
            let Some(next_target) = target_nonce.checked_add(1) else {
                // We've got contiguous nonces up to `u32::MAX`; return everything.
                return mem::take(&mut self.txs).into_values();
            };
            target_nonce = next_target;
            split_at = next_target;
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

    fn has_balance_to_cover(
        &self,
        _: &TimemarkedTransaction,
        _: &HashMap<IbcPrefixed, u128>,
    ) -> bool {
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

    /// Returns `Ok` if adding `ttx` would not break the balance precondition, i.e. enough
    /// balance to cover all transactions.
    /// Note: some implementations may clone the `current_account_balance` hashmap.
    fn has_balance_to_cover(
        &self,
        ttx: &TimemarkedTransaction,
        current_account_balance: &HashMap<IbcPrefixed, u128>,
    ) -> bool;

    /// Adds transaction to the container. Note: does NOT allow for nonce replacement.
    ///
    /// Will fail if in `SequentialNonces` mode and adding the transaction would create a nonce gap.
    /// Will fail if adding the transaction would exceed balance constraints.
    ///
    /// `current_account_nonce` should be the account's nonce in the latest chain state.
    /// `current_account_balance` should be the account's balances in the lastest chain state.
    ///
    /// Note: if the account `current_account_nonce` ever decreases, this is a logic error
    /// and could mess up the validity of `SequentialNonces` containers.
    fn add(
        &mut self,
        ttx: TimemarkedTransaction,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
    ) -> Result<(), InsertionError> {
        if self.is_at_tx_limit() {
            return Err(InsertionError::AccountSizeLimit);
        }

        if ttx.nonce() < current_account_nonce {
            return Err(InsertionError::NonceTooLow);
        }

        if let Some(existing_ttx) = self.txs().get(&ttx.nonce()) {
            return Err(if existing_ttx.id() == ttx.id() {
                InsertionError::AlreadyPresent
            } else {
                InsertionError::NonceTaken
            });
        }

        if !self.is_sequential_nonce_precondition_met(&ttx, current_account_nonce) {
            return Err(InsertionError::NonceGap);
        }

        if !self.has_balance_to_cover(&ttx, current_account_balances) {
            return Err(InsertionError::AccountBalanceTooLow);
        }

        self.txs_mut().insert(ttx.nonce(), ttx);

        Ok(())
    }

    /// Removes transactions with the given nonce and higher.
    ///
    /// Note: the given nonce is expected to be present. If it's absent, an error is logged and no
    /// transactions are removed.
    ///
    /// Returns the IDs of the removed transactions.
    fn remove(&mut self, nonce: u32) -> Vec<TransactionId> {
        if !self.txs().contains_key(&nonce) {
            error!(nonce, "transaction with given nonce not found");
            return Vec::new();
        }

        self.txs_mut()
            .split_off(&nonce)
            .values()
            .map(|ttx| *ttx.id())
            .collect()
    }

    fn contains_tx(&self, tx_id: &TransactionId) -> bool {
        self.txs().values().any(|ttx| ttx.id() == tx_id)
    }
}

/// A container used for managing pending transactions for multiple accounts.
#[derive(Clone, Debug)]
pub(super) struct PendingTransactions {
    txs: HashMap<[u8; ADDRESS_LENGTH], PendingTransactionsForAccount>,
    tx_ttl: Duration,
}

/// A container used for managing parked transactions for multiple accounts.
#[derive(Clone, Debug)]
pub(super) struct ParkedTransactions<const MAX_TX_COUNT_PER_ACCOUNT: usize> {
    txs: HashMap<[u8; ADDRESS_LENGTH], ParkedTransactionsForAccount<MAX_TX_COUNT_PER_ACCOUNT>>,
    tx_ttl: Duration,
    max_tx_count: usize,
}

impl TransactionsContainer<PendingTransactionsForAccount> for PendingTransactions {
    fn txs(&self) -> &HashMap<[u8; ADDRESS_LENGTH], PendingTransactionsForAccount> {
        &self.txs
    }

    fn txs_mut(&mut self) -> &mut HashMap<[u8; ADDRESS_LENGTH], PendingTransactionsForAccount> {
        &mut self.txs
    }

    fn tx_ttl(&self) -> Duration {
        self.tx_ttl
    }

    fn check_total_tx_count(&self) -> Result<(), InsertionError> {
        Ok(())
    }
}

impl<const MAX_TX_COUNT_PER_ACCOUNT: usize>
    TransactionsContainer<ParkedTransactionsForAccount<MAX_TX_COUNT_PER_ACCOUNT>>
    for ParkedTransactions<MAX_TX_COUNT_PER_ACCOUNT>
{
    fn txs(
        &self,
    ) -> &HashMap<[u8; ADDRESS_LENGTH], ParkedTransactionsForAccount<MAX_TX_COUNT_PER_ACCOUNT>>
    {
        &self.txs
    }

    fn txs_mut(
        &mut self,
    ) -> &mut HashMap<[u8; ADDRESS_LENGTH], ParkedTransactionsForAccount<MAX_TX_COUNT_PER_ACCOUNT>>
    {
        &mut self.txs
    }

    fn tx_ttl(&self) -> Duration {
        self.tx_ttl
    }

    fn check_total_tx_count(&self) -> Result<(), InsertionError> {
        if self.len() >= self.max_tx_count {
            return Err(InsertionError::ParkedSizeLimit);
        }
        Ok(())
    }
}

/// `TransactionsContainer` is a container used for managing transactions for multiple accounts.
pub(super) trait TransactionsContainer<T: TransactionsForAccount> {
    fn txs(&self) -> &HashMap<[u8; ADDRESS_LENGTH], T>;

    fn txs_mut(&mut self) -> &mut HashMap<[u8; ADDRESS_LENGTH], T>;

    fn tx_ttl(&self) -> Duration;

    fn check_total_tx_count(&self) -> Result<(), InsertionError>;

    /// Returns all of the currently tracked addresses.
    fn addresses<'a>(&'a self) -> impl Iterator<Item = &'a [u8; ADDRESS_LENGTH]>
    where
        T: 'a,
    {
        self.txs().keys()
    }

    /// Recosts transactions for an account.
    ///
    /// Logs an error if fails to recost a transaction.
    #[instrument(skip_all, fields(address = %telemetry::display::base64(address_bytes)))]
    async fn recost_transactions<S: accounts::StateReadExt>(
        &mut self,
        address_bytes: &[u8; ADDRESS_LENGTH],
        state: &S,
    ) {
        let Some(account) = self.txs_mut().get_mut(address_bytes) else {
            return;
        };

        for ttx in account.txs_mut().values_mut() {
            if let Err(error) = ttx.recalculate_costs(state).await {
                error!(
                    address = %telemetry::display::base64(address_bytes),
                    "failed to calculate new transaction cost when cleaning accounts: {error:#}"
                );
                continue;
            }
        }
    }

    /// Adds the transaction to the container.
    ///
    /// `current_account_nonce` should be the current nonce of the account associated with the
    /// transaction. If this ever decreases, the `TransactionsContainer` containers could become
    /// invalid.
    fn add(
        &mut self,
        ttx: TimemarkedTransaction,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
    ) -> Result<(), InsertionError> {
        self.check_total_tx_count()?;

        match self.txs_mut().entry(*ttx.address_bytes()) {
            hash_map::Entry::Occupied(entry) => {
                entry
                    .into_mut()
                    .add(ttx, current_account_nonce, current_account_balances)?;
            }
            hash_map::Entry::Vacant(entry) => {
                let mut txs = T::new();
                txs.add(ttx, current_account_nonce, current_account_balances)?;
                entry.insert(txs);
            }
        }
        Ok(())
    }

    /// Removes the given transaction and any transactions with higher nonces for the relevant
    /// account.
    ///
    /// If `checked_tx` existed, returns `Ok` with the IDs of the removed transactions. If
    /// `checked_tx` was not in the collection, it is returned via `Err`.
    fn remove(
        &mut self,
        checked_tx: Arc<CheckedTransaction>,
    ) -> Result<Vec<TransactionId>, Arc<CheckedTransaction>> {
        let address_bytes = checked_tx.address_bytes();

        // Take the collection for this account out of `self` temporarily.
        let Some(mut account_txs) = self.txs_mut().remove(address_bytes) else {
            return Err(checked_tx);
        };

        let removed = account_txs.remove(checked_tx.nonce());

        // Re-add the collection to `self` if it's not empty.
        if !account_txs.txs().is_empty() {
            let _ = self.txs_mut().insert(*address_bytes, account_txs);
        }

        if removed.is_empty() {
            return Err(checked_tx);
        }

        Ok(removed)
    }

    /// Removes all of the transactions for the given account and returns the IDs of the removed
    /// transactions.
    fn clear_account(&mut self, address_bytes: &[u8; ADDRESS_LENGTH]) -> Vec<TransactionId> {
        self.txs_mut()
            .remove(address_bytes)
            .map(|account_txs| account_txs.txs().values().map(|ttx| *ttx.id()).collect())
            .unwrap_or_default()
    }

    /// Cleans the specified account of stale and expired transactions.
    fn clean_account_stale_expired(
        &mut self,
        address_bytes: &[u8; ADDRESS_LENGTH],
        current_account_nonce: u32,
        txs_included_in_block: &HashSet<TransactionId>,
        block_height: u64,
    ) -> Vec<(TransactionId, RemovalReason)> {
        // Take the collection for this account out of `self` temporarily if it exists.
        let Some(mut account_txs) = self.txs_mut().remove(address_bytes) else {
            return Vec::new();
        };

        // clear out stale nonces
        let mut split_off = account_txs.txs_mut().split_off(&current_account_nonce);
        mem::swap(&mut split_off, account_txs.txs_mut());
        let mut removed_txs: Vec<_> = split_off
            .into_values()
            .map(|ttx| {
                if txs_included_in_block.contains(ttx.id()) {
                    // We only need to check stale transactions for inclusion, since all executed
                    // transactions will be stale
                    (*ttx.id(), RemovalReason::IncludedInBlock(block_height))
                } else {
                    (*ttx.id(), RemovalReason::NonceStale)
                }
            })
            .collect();

        // check for expired transactions
        if let Some(first_tx) = account_txs.txs_mut().first_entry() {
            if first_tx.get().is_expired(Instant::now(), self.tx_ttl()) {
                removed_txs.push((*first_tx.get().id(), RemovalReason::Expired));
                removed_txs.extend(
                    account_txs
                        .txs()
                        .values()
                        .skip(1)
                        .map(|ttx| (*ttx.id(), RemovalReason::LowerNonceInvalidated)),
                );
                account_txs.txs_mut().clear();
            }
        }

        // Re-add the collection to `self` if it's not empty.
        if !account_txs.txs().is_empty() {
            let _ = self.txs_mut().insert(*address_bytes, account_txs);
        }

        removed_txs
    }

    /// Returns the number of transactions in the container.
    fn len(&self) -> usize {
        self.txs()
            .values()
            .map(|account_txs| account_txs.txs().len())
            .sum()
    }

    fn contains_tx(&self, tx_id: &TransactionId) -> bool {
        self.txs()
            .values()
            .any(|account_txs| account_txs.contains_tx(tx_id))
    }
}

impl PendingTransactions {
    pub(super) fn new(tx_ttl: Duration) -> Self {
        PendingTransactions {
            txs: HashMap::new(),
            tx_ttl,
        }
    }

    /// Remove and return transactions that should be moved from pending to parked
    /// based on the specified account's current balances.
    pub(super) fn find_demotables(
        &mut self,
        address_bytes: &[u8; ADDRESS_LENGTH],
        current_balances: &HashMap<IbcPrefixed, u128>,
    ) -> Vec<TimemarkedTransaction> {
        // Take the collection for this account out of `self` temporarily if it exists.
        let Some(mut account) = self.txs.remove(address_bytes) else {
            return Vec::new();
        };

        let demoted = account.find_demotables(current_balances.clone());

        // Re-add the collection to `self` if it's not empty.
        if !account.txs().is_empty() {
            let _ = self.txs.insert(*address_bytes, account);
        }

        demoted
    }

    /// Returns remaining balances for an account after accounting for contained
    /// transactions' costs.
    pub(super) fn subtract_contained_costs(
        &self,
        address_bytes: &[u8; ADDRESS_LENGTH],
        mut current_balances: HashMap<IbcPrefixed, u128>,
    ) -> HashMap<IbcPrefixed, u128> {
        if let Some(account) = self.txs.get(address_bytes) {
            account.subtract_contained_costs(&mut current_balances);
        };
        current_balances
    }

    /// Returns the highest nonce for an account.
    pub(super) fn pending_nonce(&self, address_bytes: &[u8; ADDRESS_LENGTH]) -> Option<u32> {
        self.txs
            .get(address_bytes)
            .and_then(PendingTransactionsForAccount::pending_account_nonce)
    }

    /// Returns a copy of transactions and their hashes sorted by nonce difference and then time
    /// first seen.
    pub(super) fn builder_queue(&self) -> Vec<Arc<CheckedTransaction>> {
        // Used to hold the values in Vec for sorting.
        struct QueueEntry {
            checked_tx: Arc<CheckedTransaction>,
            priority: TransactionPriority,
        }

        let mut queue = Vec::with_capacity(self.len());
        // Add all transactions to the queue.
        for (address_bytes, account_txs) in &self.txs {
            let Some(current_account_nonce) = account_txs.current_account_nonce() else {
                error!(
                    address = %telemetry::display::base64(address_bytes),
                    "pending queue is empty during builder queue step"
                );
                continue;
            };

            for ttx in account_txs.txs.values() {
                let priority = match ttx.priority(current_account_nonce) {
                    Ok(priority) => priority,
                    Err(error) => {
                        // mempool could be off due to node connectivity issues
                        error!(
                            tx_id = %ttx.id(),
                            "failed to add pending tx to builder queue: {error:#}"
                        );
                        continue;
                    }
                };
                queue.push(QueueEntry {
                    checked_tx: ttx.checked_tx.clone(),
                    priority,
                });
            }
        }

        // Sort the queue and return the relevant data. Note that the sorted queue will be ordered
        // from lowest to highest priority, so we need to reverse the order before returning.
        queue.sort_unstable_by_key(|entry| entry.priority);
        queue
            .into_iter()
            .rev()
            .map(|entry| entry.checked_tx)
            .collect()
    }
}

impl<const MAX_PARKED_TXS_PER_ACCOUNT: usize> ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT> {
    pub(super) fn new(tx_ttl: Duration, max_tx_count: usize) -> Self {
        ParkedTransactions {
            txs: HashMap::new(),
            tx_ttl,
            max_tx_count,
        }
    }

    /// Removes and returns the transactions that can be promoted from parked to pending for
    /// an account. Will only return sequential nonces from `target_nonce` whose costs are
    /// covered by the `available_balance`.
    pub(super) fn find_promotables(
        &mut self,
        address_bytes: &[u8; ADDRESS_LENGTH],
        target_nonce: u32,
        available_balance: &HashMap<IbcPrefixed, u128>,
    ) -> Vec<TimemarkedTransaction> {
        // Take the collection for this account out of `self` temporarily.
        let Some(mut account_txs) = self.txs.remove(address_bytes) else {
            return Vec::new();
        };

        let removed = account_txs.find_promotables(target_nonce, available_balance.clone());

        // Re-add the collection to `self` if it's not empty.
        if !account_txs.txs().is_empty() {
            let _ = self.txs.insert(*address_bytes, account_txs);
        }

        removed.collect()
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::SigningKey,
        primitive::v1::RollupId,
        protocol::{
            fees::v1::FeeComponents,
            transaction::v1::action::{
                FeeAssetChange,
                InitBridgeAccount,
                RollupDataSubmission,
                SudoAddressChange,
            },
        },
    };
    use bytes::Bytes;

    use super::*;
    use crate::{
        checked_actions::CheckedAction,
        fees::StateWriteExt as _,
        test_utils::{
            denom_0,
            denom_1,
            denom_3,
            dummy_balances,
            dummy_tx_costs,
            CheckedTxBuilder,
            Fixture,
            ALICE,
            ALICE_ADDRESS_BYTES,
            BOB,
            BOB_ADDRESS_BYTES,
            CAROL,
            CAROL_ADDRESS,
            CAROL_ADDRESS_BYTES,
            SUDO,
        },
    };

    const MAX_PARKED_TXS_PER_ACCOUNT: usize = 15;
    const TX_TTL: Duration = Duration::from_secs(2);

    struct MockTTXBuilder<'a> {
        checked_tx_builder: CheckedTxBuilder<'a>,
        group: Option<Group>,
        cost_map: HashMap<IbcPrefixed, u128>,
    }

    impl<'a> MockTTXBuilder<'a> {
        fn new(fixture: &'a Fixture) -> Self {
            Self {
                checked_tx_builder: fixture.checked_tx_builder().with_signer(ALICE.clone()),
                group: None,
                cost_map: dummy_tx_costs(0, 0, 0),
            }
        }

        fn nonce(mut self, nonce: u32) -> Self {
            self.checked_tx_builder = self.checked_tx_builder.with_nonce(nonce);
            self
        }

        fn signer(mut self, signer: SigningKey) -> Self {
            self.checked_tx_builder = self.checked_tx_builder.with_signer(signer);
            self
        }

        fn group(mut self, group: Group) -> Self {
            self.group = Some(group);
            self
        }

        fn cost_map(mut self, cost_map: HashMap<IbcPrefixed, u128>) -> Self {
            self.cost_map = cost_map;
            self
        }

        async fn build(self) -> TimemarkedTransaction {
            let tx = match self.group {
                Some(Group::UnbundleableSudo) => {
                    self.checked_tx_builder
                        .with_action(SudoAddressChange {
                            new_address: *CAROL_ADDRESS,
                        })
                        .build()
                        .await
                }
                Some(Group::BundleableSudo) => {
                    self.checked_tx_builder
                        .with_action(FeeAssetChange::Addition(denom_0()))
                        .build()
                        .await
                }
                Some(Group::UnbundleableGeneral) => {
                    self.checked_tx_builder
                        .with_action(InitBridgeAccount {
                            rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                            asset: denom_0(),
                            fee_asset: denom_0(),
                            sudo_address: None,
                            withdrawer_address: None,
                        })
                        .build()
                        .await
                }
                Some(Group::BundleableGeneral) => {
                    self.checked_tx_builder
                        .with_action(RollupDataSubmission {
                            rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                            data: Bytes::from_static(&[0x99]),
                            fee_asset: denom_0(),
                        })
                        .build()
                        .await
                }
                None => self.checked_tx_builder.build().await,
            };
            if let Some(group) = self.group {
                assert_eq!(group, tx.group());
            }
            TimemarkedTransaction::new(tx, self.cost_map)
        }
    }

    #[tokio::test]
    async fn transaction_priority_should_error_if_invalid() {
        let fixture = Fixture::default_initialized().await;
        let ttx = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let priority = ttx.priority(1);

        assert!(priority
            .unwrap_err()
            .to_string()
            .contains("less than current account nonce"));
    }

    // From https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
    #[test]
    fn transaction_priority_comparisons_should_be_consistent_action_group() {
        let instant = Instant::now();

        let bundleable_general = TransactionPriority {
            group: Group::BundleableGeneral,
            nonce_diff: 0,
            time_first_seen: instant,
        };
        let unbundleable_general = TransactionPriority {
            group: Group::UnbundleableGeneral,
            nonce_diff: 0,
            time_first_seen: instant,
        };
        let bundleable_sudo = TransactionPriority {
            group: Group::BundleableSudo,
            nonce_diff: 0,
            time_first_seen: instant,
        };
        let unbundleable_sudo = TransactionPriority {
            group: Group::UnbundleableSudo,
            nonce_diff: 0,
            time_first_seen: instant,
        };

        // partial_cmp
        assert!(bundleable_general.partial_cmp(&bundleable_general) == Some(Ordering::Equal));
        assert!(bundleable_general.partial_cmp(&unbundleable_general) == Some(Ordering::Greater));
        assert!(bundleable_general.partial_cmp(&bundleable_sudo) == Some(Ordering::Greater));
        assert!(bundleable_general.partial_cmp(&unbundleable_sudo) == Some(Ordering::Greater));

        assert!(unbundleable_general.partial_cmp(&bundleable_general) == Some(Ordering::Less));
        assert!(unbundleable_general.partial_cmp(&unbundleable_general) == Some(Ordering::Equal));
        assert!(unbundleable_general.partial_cmp(&bundleable_sudo) == Some(Ordering::Greater));
        assert!(unbundleable_general.partial_cmp(&unbundleable_sudo) == Some(Ordering::Greater));

        assert!(bundleable_sudo.partial_cmp(&bundleable_general) == Some(Ordering::Less));
        assert!(bundleable_sudo.partial_cmp(&unbundleable_general) == Some(Ordering::Less));
        assert!(bundleable_sudo.partial_cmp(&bundleable_sudo) == Some(Ordering::Equal));
        assert!(bundleable_sudo.partial_cmp(&unbundleable_sudo) == Some(Ordering::Greater));

        assert!(unbundleable_sudo.partial_cmp(&bundleable_general) == Some(Ordering::Less));
        assert!(unbundleable_sudo.partial_cmp(&unbundleable_general) == Some(Ordering::Less));
        assert!(unbundleable_sudo.partial_cmp(&bundleable_sudo) == Some(Ordering::Less));
        assert!(unbundleable_sudo.partial_cmp(&unbundleable_sudo) == Some(Ordering::Equal));

        // equal
        assert!(bundleable_general == bundleable_general);
        assert!(unbundleable_general == unbundleable_general);
        assert!(bundleable_sudo == bundleable_sudo);
        assert!(unbundleable_sudo == unbundleable_sudo);

        // greater than
        assert!(bundleable_general > unbundleable_general);
        assert!(bundleable_general > bundleable_sudo);
        assert!(bundleable_general > unbundleable_sudo);

        assert!(unbundleable_general > bundleable_sudo);
        assert!(unbundleable_general > unbundleable_sudo);

        assert!(bundleable_sudo > unbundleable_sudo);

        // greater than or equal to
        assert!(bundleable_general >= bundleable_general);
        assert!(bundleable_general >= unbundleable_general);
        assert!(bundleable_general >= bundleable_sudo);
        assert!(bundleable_general >= unbundleable_sudo);

        assert!(unbundleable_general >= unbundleable_general);
        assert!(unbundleable_general >= bundleable_sudo);
        assert!(unbundleable_general >= unbundleable_sudo);

        assert!(bundleable_sudo >= bundleable_sudo);
        assert!(bundleable_sudo >= unbundleable_sudo);

        assert!(unbundleable_sudo >= unbundleable_sudo);

        // less than
        assert!(unbundleable_sudo < bundleable_sudo);
        assert!(unbundleable_sudo < unbundleable_general);
        assert!(unbundleable_sudo < bundleable_general);

        assert!(bundleable_sudo < bundleable_general);
        assert!(bundleable_sudo < unbundleable_general);

        assert!(unbundleable_general < bundleable_general);

        // less than or equal to
        assert!(unbundleable_sudo <= unbundleable_sudo);
        assert!(unbundleable_sudo <= bundleable_sudo);
        assert!(unbundleable_sudo <= unbundleable_general);
        assert!(unbundleable_general <= bundleable_general);

        assert!(bundleable_sudo <= bundleable_sudo);
        assert!(bundleable_sudo <= bundleable_general);
        assert!(bundleable_sudo <= unbundleable_general);

        assert!(unbundleable_general <= unbundleable_general);
        assert!(unbundleable_general <= bundleable_general);

        assert!(bundleable_general <= bundleable_general);

        // not equal
        assert!(bundleable_general != unbundleable_general);
        assert!(bundleable_general != unbundleable_sudo);
        assert!(bundleable_general != bundleable_sudo);
        assert!(unbundleable_general != bundleable_sudo);
        assert!(unbundleable_general != unbundleable_sudo);
        assert!(bundleable_sudo != unbundleable_sudo);
    }

    #[test]
    fn transaction_priority_comparisons_should_be_consistent_nonce_diff() {
        let instant = Instant::now();

        let high = TransactionPriority {
            group: Group::BundleableGeneral,
            nonce_diff: 0,
            time_first_seen: instant,
        };
        let low = TransactionPriority {
            group: Group::BundleableGeneral,
            nonce_diff: 1,
            time_first_seen: instant,
        };

        assert!(high.partial_cmp(&high) == Some(Ordering::Equal));
        assert!(high.partial_cmp(&low) == Some(Ordering::Greater));
        assert!(low.partial_cmp(&high) == Some(Ordering::Less));

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal)
        assert!(high == high); // Some(Equal)
        assert!(high != low); // Some(Greater)
        assert!(low != high); // Some(Less)

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)
        assert!(low < high); // Some(Less)
        assert!(high >= high); // Some(Equal)
        assert!(high >= low); // Some(Greater)

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert!(high > low); // Some(Greater)
        assert!(high <= high); // Some(Equal)
        assert!(low <= high); // Some(Less)

        // 4. a <= b if and only if a < b || a == b
        assert!(low <= high); // a < b
        assert!(high <= high); // a == b
        assert!(high > low); // !(b <= a)

        // 5. a >= b if and only if a > b || a == b
        assert!(high >= low); // a > b
        assert!(high >= high); // a == b
        assert!(low < high); // !(b >= a)

        // 6. a != b if and only if !(a == b)
        assert!(high != low); // asserted !(high == low) above
        assert!(low != high); // asserted !(low == high) above
        assert!(high == high); // asserted high == high above
    }

    // From https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
    #[test]
    fn transaction_priority_comparisons_should_be_consistent_time_gap() {
        let high = TransactionPriority {
            group: Group::BundleableGeneral,
            nonce_diff: 0,
            time_first_seen: Instant::now(),
        };
        let low = TransactionPriority {
            group: Group::BundleableGeneral,
            nonce_diff: 0,
            time_first_seen: Instant::now() + Duration::from_micros(10),
        };

        assert!(high.partial_cmp(&high) == Some(Ordering::Equal));
        assert!(high.partial_cmp(&low) == Some(Ordering::Greater));
        assert!(low.partial_cmp(&high) == Some(Ordering::Less));

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal)
        assert!(high == high); // Some(Equal)
        assert!(high != low); // Some(Greater)
        assert!(low != high); // Some(Less)

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)
        assert!(low < high); // Some(Less)
        assert!(high >= high); // Some(Equal)
        assert!(high >= low); // Some(Greater)

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert!(high > low); // Some(Greater)
        assert!(high <= high); // Some(Equal)
        assert!(low <= high); // Some(Less)

        // 4. a <= b if and only if a < b || a == b
        assert!(low <= high); // a < b
        assert!(high <= high); // a == b
        assert!(high > low); // !(b <= a)

        // 5. a >= b if and only if a > b || a == b
        assert!(high >= low); // a > b
        assert!(high >= high); // a == b
        assert!(low < high); // !(low >= high)

        // 6. a != b if and only if !(a == b)
        assert!(high != low); // asserted !(high == low) above
        assert!(low != high); // asserted !(low == high) above
        assert!(high == high); // asserted !(high != high) above
    }

    #[tokio::test]
    async fn parked_transactions_for_account_add() {
        let fixture = Fixture::default_initialized().await;
        let mut parked_txs = ParkedTransactionsForAccount::<MAX_PARKED_TXS_PER_ACCOUNT>::new();

        // transactions to add
        let ttx_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .cost_map(dummy_tx_costs(10, 0, 0))
            .build()
            .await;
        let ttx_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .cost_map(dummy_tx_costs(0, 10, 0))
            .build()
            .await;
        let ttx_5 = MockTTXBuilder::new(&fixture)
            .nonce(5)
            .cost_map(dummy_tx_costs(0, 0, 100))
            .build()
            .await;

        // note account doesn't have balance to cover any of them
        let account_balances = dummy_balances(1, 1);

        let current_account_nonce = 2;
        parked_txs
            .add(ttx_3.clone(), current_account_nonce, &account_balances)
            .unwrap();
        assert!(parked_txs.contains_tx(ttx_3.checked_tx.id()));
        assert_eq!(
            parked_txs
                .add(ttx_3, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::AlreadyPresent
        );

        // add gapped transaction
        parked_txs
            .add(ttx_5, current_account_nonce, &account_balances)
            .unwrap();

        // fail adding too low nonce
        assert_eq!(
            parked_txs
                .add(ttx_1, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::NonceTooLow
        );
    }

    #[tokio::test]
    async fn parked_transactions_for_account_size_limit() {
        let fixture = Fixture::default_initialized().await;
        let mut parked_txs = ParkedTransactionsForAccount::<2>::new();

        // transactions to add
        let ttx_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_3 = MockTTXBuilder::new(&fixture).nonce(3).build().await;
        let ttx_5 = MockTTXBuilder::new(&fixture).nonce(5).build().await;
        let account_balances = dummy_balances(1, 1);

        let current_account_nonce = 0;
        parked_txs
            .add(ttx_3.clone(), current_account_nonce, &account_balances)
            .unwrap();
        parked_txs
            .add(ttx_5, current_account_nonce, &account_balances)
            .unwrap();

        // fail with size limit hit
        assert_eq!(
            parked_txs
                .add(ttx_1, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::AccountSizeLimit
        );
    }

    #[tokio::test]
    async fn pending_transactions_for_account_add() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactionsForAccount::new();

        // transactions to add, not testing balances in this unit test
        let ttx_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_2 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let ttx_3 = MockTTXBuilder::new(&fixture).nonce(3).build().await;

        let account_balances = dummy_balances(1, 1);

        let current_account_nonce = 1;

        // too low nonces not added
        assert_eq!(
            pending_txs
                .add(ttx_0, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::NonceTooLow
        );
        assert!(pending_txs.txs().is_empty());

        // too high nonces with empty container not added
        assert_eq!(
            pending_txs
                .add(ttx_2.clone(), current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::NonceGap
        );
        assert!(pending_txs.txs().is_empty());

        // add ok
        pending_txs
            .add(ttx_1.clone(), current_account_nonce, &account_balances)
            .unwrap();
        assert_eq!(
            pending_txs
                .add(ttx_1, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::AlreadyPresent
        );

        // gapped transaction not allowed
        assert_eq!(
            pending_txs
                .add(ttx_3, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::NonceGap
        );

        // can add consecutive
        pending_txs
            .add(ttx_2, current_account_nonce, &account_balances)
            .unwrap();
    }

    #[tokio::test]
    async fn pending_transactions_for_account_add_balances() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactionsForAccount::new();

        // transactions to add, testing balances
        let ttx_0_too_expensive_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .cost_map(dummy_tx_costs(11, 0, 0))
            .build()
            .await;
        let ttx_0_too_expensive_1 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .cost_map(dummy_tx_costs(0, 0, 1))
            .build()
            .await;
        let ttx_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .cost_map(dummy_tx_costs(10, 0, 0))
            .build()
            .await;
        let ttx_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .cost_map(dummy_tx_costs(0, 10, 0))
            .build()
            .await;
        let ttx_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .cost_map(dummy_tx_costs(0, 8, 0))
            .build()
            .await;
        let ttx_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .cost_map(dummy_tx_costs(0, 2, 0))
            .build()
            .await;
        let ttx_4 = MockTTXBuilder::new(&fixture)
            .nonce(4)
            .cost_map(dummy_tx_costs(0, 0, 1))
            .build()
            .await;

        let account_balances = dummy_balances(10, 20);
        let current_account_nonce = 0;

        // transaction exceeding account balances (asset present in balances) not allowed
        assert_eq!(
            pending_txs
                .add(
                    ttx_0_too_expensive_0,
                    current_account_nonce,
                    &account_balances,
                )
                .unwrap_err(),
            InsertionError::AccountBalanceTooLow
        );
        assert!(pending_txs.txs().is_empty());

        // transaction exceeding account balances (asset NOT present in balances) not allowed
        assert_eq!(
            pending_txs
                .add(
                    ttx_0_too_expensive_1,
                    current_account_nonce,
                    &account_balances,
                )
                .unwrap_err(),
            InsertionError::AccountBalanceTooLow
        );
        assert!(pending_txs.txs().is_empty());

        // transactions under account cost allowed
        pending_txs
            .add(ttx_0, current_account_nonce, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_1.clone(), current_account_nonce, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_2.clone(), current_account_nonce, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_3.clone(), current_account_nonce, &account_balances)
            .unwrap();

        assert_eq!(pending_txs.txs().len(), 4);

        // check that remaining balances are zero
        let mut remaining_balances = account_balances.clone();
        pending_txs.subtract_contained_costs(&mut remaining_balances);

        for (asset, balance) in remaining_balances {
            if asset != denom_3().to_ibc_prefixed() {
                assert_eq!(balance, 0, "balance should have been consumed");
            }
        }

        // cost exceeding when considering already contained transactions not allowed
        assert_eq!(
            pending_txs
                .add(ttx_4, current_account_nonce, &account_balances)
                .unwrap_err(),
            InsertionError::AccountBalanceTooLow
        );
    }

    #[tokio::test]
    async fn transactions_for_account_remove() {
        let fixture = Fixture::default_initialized().await;
        let mut account_txs = PendingTransactionsForAccount::new();

        // transactions to add
        let ttx_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_2 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let ttx_3 = MockTTXBuilder::new(&fixture).nonce(3).build().await;
        let account_balances = dummy_balances(1, 1);

        account_txs
            .add(ttx_0.clone(), 0, &account_balances)
            .unwrap();
        account_txs
            .add(ttx_1.clone(), 0, &account_balances)
            .unwrap();
        account_txs
            .add(ttx_2.clone(), 0, &account_balances)
            .unwrap();
        account_txs
            .add(ttx_3.clone(), 0, &account_balances)
            .unwrap();

        // remove from end will only remove end
        assert_eq!(
            account_txs.remove(3),
            vec![*ttx_3.checked_tx.id()],
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
            vec![
                *ttx_0.checked_tx.id(),
                *ttx_1.checked_tx.id(),
                *ttx_2.checked_tx.id()
            ],
            "three transactions should've been removed"
        );
        assert!(account_txs.txs().is_empty());
    }

    #[tokio::test]
    async fn pending_transactions_for_account_pending_account_nonce() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactionsForAccount::new();

        // no transactions ok
        assert!(
            pending_txs.pending_account_nonce().is_none(),
            "no transactions will return None"
        );

        // transactions to add
        let ttx_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_2 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let account_balances = dummy_balances(1, 1);

        pending_txs.add(ttx_0, 0, &account_balances).unwrap();
        pending_txs.add(ttx_1, 0, &account_balances).unwrap();
        pending_txs.add(ttx_2, 0, &account_balances).unwrap();

        // will return last transaction
        assert_eq!(
            pending_txs.pending_account_nonce(),
            Some(3),
            "account nonce after all pending have executed should be returned"
        );
    }

    #[tokio::test]
    async fn transactions_container_add() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        // transactions to add to accounts
        let ttx_s0_0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        // Same nonce and signer as `ttx_s0_0_0`, but different action.
        let ttx_s0_0_1 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .group(Group::UnbundleableGeneral)
            .build()
            .await;
        let ttx_s0_2_0 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let ttx_s1_0_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .build()
            .await;
        let account_balances = dummy_balances(1, 1);

        // transactions to add for account 1

        // initially no accounts should exist
        assert!(
            pending_txs.txs.is_empty(),
            "no accounts should exist at first"
        );

        // adding too low nonce shouldn't create account
        assert_eq!(
            pending_txs
                .add(ttx_s0_0_0.clone(), 1, &account_balances)
                .unwrap_err(),
            InsertionError::NonceTooLow,
            "shouldn't be able to add nonce too low transaction"
        );
        assert!(
            pending_txs.txs.is_empty(),
            "failed adds to new accounts shouldn't create account"
        );

        // add one transaction
        pending_txs
            .add(ttx_s0_0_0.clone(), 0, &account_balances)
            .unwrap();
        assert_eq!(pending_txs.txs.len(), 1, "one account should exist");

        // re-adding transaction should fail
        assert_eq!(
            pending_txs
                .add(ttx_s0_0_0, 0, &account_balances)
                .unwrap_err(),
            InsertionError::AlreadyPresent,
            "re-adding same transaction should fail"
        );

        // nonce replacement fails
        assert_eq!(
            pending_txs
                .add(ttx_s0_0_1, 0, &account_balances)
                .unwrap_err(),
            InsertionError::NonceTaken,
            "nonce replacement not supported"
        );

        // nonce gaps not supported
        assert_eq!(
            pending_txs
                .add(ttx_s0_2_0, 0, &account_balances)
                .unwrap_err(),
            InsertionError::NonceGap,
            "gapped nonces in pending transactions not allowed"
        );

        // add transactions for account 2
        pending_txs.add(ttx_s1_0_0, 0, &account_balances).unwrap();

        // check internal structures
        assert_eq!(pending_txs.txs.len(), 2, "two accounts should exist");
        assert_eq!(
            pending_txs
                .txs
                .get(&*ALICE_ADDRESS_BYTES)
                .unwrap()
                .txs()
                .len(),
            1,
            "one transaction should be in the original account"
        );
        assert_eq!(
            pending_txs
                .txs
                .get(&*BOB_ADDRESS_BYTES)
                .unwrap()
                .txs()
                .len(),
            1,
            "one transaction should be in the second account"
        );
        assert_eq!(
            pending_txs.len(),
            2,
            "should only have two transactions tracked"
        );
    }

    #[tokio::test]
    async fn transactions_container_remove() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to accounts
        let ttx_s0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_s1_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s1_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .signer(BOB.clone())
            .build()
            .await;
        let account_balances = dummy_balances(1, 1);

        // remove on empty returns the tx in Err variant.
        assert!(
            pending_txs.remove(ttx_s0_0.checked_tx.clone()).is_err(),
            "zero transactions should be removed from non existing accounts"
        );

        // add transactions
        pending_txs
            .add(ttx_s0_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s0_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_1.clone(), 0, &account_balances)
            .unwrap();

        // remove should remove tx and higher
        assert_eq!(
            pending_txs.remove(ttx_s0_0.checked_tx.clone()).unwrap(),
            vec![*ttx_s0_0.checked_tx.id(), *ttx_s0_1.checked_tx.id()],
            "rest of transactions for account should be removed when targeting bottom nonce"
        );
        assert_eq!(pending_txs.txs.len(), 1, "empty account should be removed");
        assert_eq!(
            pending_txs.len(),
            2,
            "should only have two transactions tracked"
        );
        assert!(
            pending_txs.contains_tx(ttx_s1_0.checked_tx.id()),
            "other account should be untouched"
        );
        assert!(
            pending_txs.contains_tx(ttx_s1_1.checked_tx.id()),
            "other account should be untouched"
        );
    }

    #[tokio::test]
    async fn transactions_container_clear_account() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to accounts
        let ttx_s0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_s1_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .build()
            .await;
        let account_balances = dummy_balances(1, 1);

        // clear all on empty returns zero
        assert!(
            pending_txs.clear_account(&ALICE_ADDRESS_BYTES).is_empty(),
            "zero transactions should be removed from clearing non existing accounts"
        );

        // add transactions
        pending_txs
            .add(ttx_s0_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s0_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_0.clone(), 0, &account_balances)
            .unwrap();

        // clear should return all transactions
        assert_eq!(
            pending_txs.clear_account(&ALICE_ADDRESS_BYTES),
            vec![*ttx_s0_0.checked_tx.id(), *ttx_s0_1.checked_tx.id()],
            "all transactions should be returned from clearing account"
        );

        assert_eq!(pending_txs.txs.len(), 1, "empty account should be removed");
        assert_eq!(
            pending_txs.len(),
            1,
            "should only have one transaction tracked"
        );
        assert!(
            pending_txs.contains_tx(ttx_s1_0.checked_tx.id()),
            "other account should be untouched"
        );
    }

    #[tokio::test]
    async fn transactions_container_recost_transactions() {
        let mut fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let account_balances = dummy_balances(1, 1);

        // transaction to add to account
        let ttx = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        pending_txs.add(ttx.clone(), 0, &account_balances).unwrap();
        assert_eq!(
            pending_txs
                .txs
                .get(&*ALICE_ADDRESS_BYTES)
                .unwrap()
                .txs
                .get(&0)
                .unwrap()
                .costs
                .get(&denom_0().to_ibc_prefixed())
                .unwrap(),
            &0,
            "cost initially should be zero"
        );

        // update the fees for `RollupDataSubmission` and recost transactions
        let base_fee = 1000;
        let multiplier = 2000;
        fixture
            .state_mut()
            .put_fees(FeeComponents::<RollupDataSubmission>::new(
                base_fee, multiplier,
            ))
            .unwrap();
        pending_txs
            .recost_transactions(&ALICE_ADDRESS_BYTES, fixture.state())
            .await;

        // transaction should have been recosted
        let rollup_data_len = match &ttx.checked_tx.checked_actions()[0] {
            CheckedAction::RollupDataSubmission(checked_action) => {
                u128::try_from(checked_action.action().data.len()).unwrap()
            }
            _ => panic!("should be rollup data submission"),
        };
        let expected_cost = base_fee
            .checked_add(multiplier.checked_mul(rollup_data_len).unwrap())
            .unwrap();

        assert_eq!(
            pending_txs
                .txs
                .get(&*ALICE_ADDRESS_BYTES)
                .unwrap()
                .txs
                .get(&0)
                .unwrap()
                .costs
                .get(&denom_0().to_ibc_prefixed())
                .unwrap(),
            &expected_cost,
            "cost should be updated"
        );
    }

    #[tokio::test]
    #[expect(clippy::too_many_lines, reason = "it's a test")]
    async fn transactions_container_clean_account_stale_expired_and_included() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 9;

        let fixture = Fixture::default_initialized().await;

        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to accounts
        let ttx_s0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_s0_2 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let ttx_s1_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s1_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s1_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s2_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(CAROL.clone())
            .build()
            .await;
        let ttx_s2_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .signer(CAROL.clone())
            .build()
            .await;
        let ttx_s2_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .signer(CAROL.clone())
            .build()
            .await;
        let account_balances = dummy_balances(1, 1);

        // add transactions
        pending_txs
            .add(ttx_s0_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s0_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s0_2.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_2.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s2_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s2_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s2_2.clone(), 0, &account_balances)
            .unwrap();

        // clean accounts
        // should pop none from signing_address_0, one from signing_address_1, and all from
        // signing_address_2
        let mut removed_txs =
            pending_txs.clean_account_stale_expired(&ALICE_ADDRESS_BYTES, 0, &HashSet::new(), 0);
        removed_txs.extend(pending_txs.clean_account_stale_expired(
            &BOB_ADDRESS_BYTES,
            1,
            &HashSet::new(),
            0,
        ));
        removed_txs.extend(pending_txs.clean_account_stale_expired(
            &CAROL_ADDRESS_BYTES,
            4,
            // should remove transactions 0 and 1 with `RemovalReason::Indluded(9)`
            &{
                let mut included_txs = HashSet::new();
                included_txs.insert(*ttx_s2_0.id());
                included_txs.insert(*ttx_s2_1.id());
                included_txs
            },
            INCLUDED_TX_BLOCK_NUMBER,
        ));

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
        assert!(pending_txs.contains_tx(ttx_s0_0.checked_tx.id()));
        assert!(pending_txs.contains_tx(ttx_s0_1.checked_tx.id()));
        assert!(pending_txs.contains_tx(ttx_s0_2.checked_tx.id()));
        assert!(pending_txs.contains_tx(ttx_s1_1.checked_tx.id()));
        assert!(pending_txs.contains_tx(ttx_s1_2.checked_tx.id()));

        assert_eq!(
            pending_txs
                .txs
                .get(&*ALICE_ADDRESS_BYTES)
                .unwrap()
                .txs()
                .len(),
            3
        );
        assert_eq!(
            pending_txs
                .txs
                .get(&*BOB_ADDRESS_BYTES)
                .unwrap()
                .txs()
                .len(),
            2
        );
        for (tx_id, reason) in removed_txs {
            if tx_id == *ttx_s2_0.id() || tx_id == *ttx_s2_1.id() {
                assert!(
                    matches!(
                        reason,
                        RemovalReason::IncludedInBlock(INCLUDED_TX_BLOCK_NUMBER)
                    ),
                    "removal reason should be included(9)"
                );
            } else {
                assert_eq!(
                    reason,
                    RemovalReason::NonceStale,
                    "removal reason should be stale nonce"
                );
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn transactions_container_clean_accounts_expired_transactions() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let account_balances = dummy_balances(1, 1);

        // transactions to add to accounts
        let ttx_s0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;

        // pass time to make first transaction stale
        tokio::time::advance(TX_TTL.saturating_add(Duration::from_nanos(1))).await;

        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_s1_0 = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .build()
            .await;

        // add transactions
        pending_txs
            .add(ttx_s0_0.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s0_1.clone(), 0, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_0.clone(), 0, &account_balances)
            .unwrap();

        // clean accounts, all nonces should be valid
        let mut removed_txs =
            pending_txs.clean_account_stale_expired(&ALICE_ADDRESS_BYTES, 0, &HashSet::new(), 1);
        removed_txs.extend(pending_txs.clean_account_stale_expired(
            &BOB_ADDRESS_BYTES,
            0,
            &HashSet::new(),
            1,
        ));

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
            pending_txs.contains_tx(ttx_s1_0.checked_tx.id()),
            "not expired account should be untouched"
        );

        // check removal reasons
        assert_eq!(
            removed_txs[0],
            (*ttx_s0_0.checked_tx.id(), RemovalReason::Expired),
            "first should be first pushed tx with removal reason as expired"
        );
        assert_eq!(
            removed_txs[1],
            (
                *ttx_s0_1.checked_tx.id(),
                RemovalReason::LowerNonceInvalidated
            ),
            "second should be second added tx with removal reason as lower nonce invalidation"
        );
    }

    #[tokio::test]
    async fn pending_transactions_pending_nonce() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);
        let account_balances = dummy_balances(1, 1);

        // transactions to add for account 0
        let ttx_s0_0 = MockTTXBuilder::new(&fixture).nonce(0).build().await;
        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;

        pending_txs.add(ttx_s0_0, 0, &account_balances).unwrap();
        pending_txs.add(ttx_s0_1, 0, &account_balances).unwrap();

        // empty account returns zero
        assert!(
            pending_txs.pending_nonce(&BOB_ADDRESS_BYTES).is_none(),
            "empty account should return None"
        );

        // non empty account returns highest nonce
        assert_eq!(
            pending_txs.pending_nonce(&ALICE_ADDRESS_BYTES),
            Some(2),
            "should return pending account nonce"
        );
    }

    #[tokio::test]
    async fn pending_transactions_builder_queue() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to accounts
        let ttx_s0_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_s1_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s1_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .signer(BOB.clone())
            .build()
            .await;
        let ttx_s1_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .signer(BOB.clone())
            .build()
            .await;
        let account_balances = dummy_balances(1, 1);

        // add transactions
        pending_txs
            .add(ttx_s0_1.clone(), 1, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_1.clone(), 1, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_2.clone(), 1, &account_balances)
            .unwrap();
        pending_txs
            .add(ttx_s1_3.clone(), 1, &account_balances)
            .unwrap();

        // get builder queue - should return all transactions from Alice and Bob
        let builder_queue = pending_txs.builder_queue();
        assert_eq!(
            builder_queue.len(),
            4,
            "four transactions should've been popped"
        );

        // check that the transactions are in the expected order
        let first_tx_id = builder_queue[0].id();
        assert_eq!(
            first_tx_id,
            ttx_s0_1.checked_tx.id(),
            "expected earliest transaction with lowest nonce difference (0) to be first"
        );
        let second_tx_id = builder_queue[1].id();
        assert_eq!(
            second_tx_id,
            ttx_s1_1.checked_tx.id(),
            "expected other low nonce diff (0) to be second"
        );
        let third_tx_id = builder_queue[2].id();
        assert_eq!(
            third_tx_id,
            ttx_s1_2.checked_tx.id(),
            "expected middle nonce diff (1) to be third"
        );
        let fourth_tx_id = builder_queue[3].id();
        assert_eq!(
            fourth_tx_id,
            ttx_s1_3.checked_tx.id(),
            "expected highest nonce diff (2) to be last"
        );

        // ensure transactions not removed
        assert_eq!(
            pending_txs.len(),
            4,
            "no transactions should've been removed"
        );
    }

    #[tokio::test]
    async fn parked_transactions_find_promotables() {
        let fixture = Fixture::default_initialized().await;
        let mut parked_txs = ParkedTransactions::<MAX_PARKED_TXS_PER_ACCOUNT>::new(TX_TTL, 100);

        // transactions to add to accounts
        let ttx_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .cost_map(dummy_tx_costs(10, 0, 0))
            .build()
            .await;
        let ttx_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .cost_map(dummy_tx_costs(5, 2, 0))
            .build()
            .await;
        let ttx_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .cost_map(dummy_tx_costs(1, 0, 0))
            .build()
            .await;
        let remaining_balances = dummy_balances(15, 2);

        // add transactions
        parked_txs
            .add(ttx_1.clone(), 0, &remaining_balances)
            .unwrap();
        parked_txs
            .add(ttx_2.clone(), 0, &remaining_balances)
            .unwrap();
        parked_txs
            .add(ttx_3.clone(), 0, &remaining_balances)
            .unwrap();

        // none should be returned on nonce gap
        let promotables = parked_txs.find_promotables(&ALICE_ADDRESS_BYTES, 0, &remaining_balances);
        assert_eq!(promotables.len(), 0);

        // only first two transactions should be returned
        let promotables = parked_txs.find_promotables(&ALICE_ADDRESS_BYTES, 1, &remaining_balances);
        assert_eq!(promotables.len(), 2);
        assert_eq!(promotables[0].nonce(), 1);
        assert_eq!(promotables[1].nonce(), 2);
        assert_eq!(
            parked_txs.len(),
            1,
            "promoted transactions should've been removed from container"
        );

        // empty account should be removed
        // remove last
        parked_txs.find_promotables(&ALICE_ADDRESS_BYTES, 3, &remaining_balances);
        assert_eq!(
            parked_txs.addresses().count(),
            0,
            "empty account should've been removed from container"
        );
    }

    #[tokio::test]
    async fn pending_transactions_find_demotables() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to account
        let ttx_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .cost_map(dummy_tx_costs(5, 0, 0))
            .build()
            .await;
        let ttx_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .cost_map(dummy_tx_costs(0, 5, 0))
            .build()
            .await;
        let ttx_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .cost_map(dummy_tx_costs(5, 0, 0))
            .build()
            .await;
        let ttx_4 = MockTTXBuilder::new(&fixture)
            .nonce(4)
            .cost_map(dummy_tx_costs(0, 5, 0))
            .build()
            .await;
        let account_balances_full = dummy_balances(100, 100);

        // add transactions
        pending_txs
            .add(ttx_1.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_2.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_3.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_4.clone(), 1, &account_balances_full)
            .unwrap();

        // demote none
        let demotables: Vec<TimemarkedTransaction> =
            pending_txs.find_demotables(&ALICE_ADDRESS_BYTES, &account_balances_full);
        assert_eq!(demotables.len(), 0);

        // demote last
        let account_balances_demotion = dummy_balances(100, 9);
        let demotables =
            pending_txs.find_demotables(&ALICE_ADDRESS_BYTES, &account_balances_demotion);
        assert_eq!(demotables.len(), 1);
        assert_eq!(demotables[0].nonce(), 4);

        // demote multiple
        let account_balances_demotion = dummy_balances(100, 4);
        let demotables =
            pending_txs.find_demotables(&ALICE_ADDRESS_BYTES, &account_balances_demotion);
        assert_eq!(demotables.len(), 2);
        assert_eq!(demotables[0].nonce(), 2);

        // demote rest
        let account_balances_demotion = dummy_balances(0, 5);
        let demotables =
            pending_txs.find_demotables(&ALICE_ADDRESS_BYTES, &account_balances_demotion);
        assert_eq!(demotables.len(), 1);
        assert_eq!(demotables[0].nonce(), 1);

        // empty account removed
        assert_eq!(
            pending_txs.addresses().count(),
            0,
            "empty account should've been removed from container"
        );
    }

    #[tokio::test]
    async fn pending_transactions_remaining_account_balances() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // transactions to add to account
        let ttx_1 = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .cost_map(dummy_tx_costs(6, 0, 0))
            .build()
            .await;
        let ttx_2 = MockTTXBuilder::new(&fixture)
            .nonce(2)
            .cost_map(dummy_tx_costs(0, 5, 0))
            .build()
            .await;
        let ttx_3 = MockTTXBuilder::new(&fixture)
            .nonce(3)
            .cost_map(dummy_tx_costs(6, 0, 0))
            .build()
            .await;
        let ttx_4 = MockTTXBuilder::new(&fixture)
            .nonce(4)
            .cost_map(dummy_tx_costs(0, 5, 0))
            .build()
            .await;
        let account_balances_full = dummy_balances(100, 100);

        // add transactions
        pending_txs
            .add(ttx_1.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_2.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_3.clone(), 1, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_4.clone(), 1, &account_balances_full)
            .unwrap();

        // get balances
        let remaining_balances =
            pending_txs.subtract_contained_costs(&ALICE_ADDRESS_BYTES, account_balances_full);
        assert_eq!(
            remaining_balances
                .get(&denom_0().to_ibc_prefixed())
                .unwrap(),
            &88
        );
        assert_eq!(
            remaining_balances
                .get(&denom_1().to_ibc_prefixed())
                .unwrap(),
            &90
        );
    }

    #[tokio::test]
    async fn builder_queue_should_be_sorted_by_action_group_type() {
        let fixture = Fixture::default_initialized().await;
        let mut pending_txs = PendingTransactions::new(TX_TTL);

        // create transactions in reverse order
        let ttx_unbundleable_sudo = MockTTXBuilder::new(&fixture)
            .nonce(1)
            .signer(SUDO.clone())
            .group(Group::UnbundleableSudo)
            .build()
            .await;
        let ttx_bundleable_sudo = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(SUDO.clone())
            .group(Group::BundleableSudo)
            .build()
            .await;
        let ttx_unbundleable_general = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(BOB.clone())
            .group(Group::UnbundleableGeneral)
            .build()
            .await;
        let ttx_bundleable_general = MockTTXBuilder::new(&fixture)
            .nonce(0)
            .signer(CAROL.clone())
            .group(Group::BundleableGeneral)
            .build()
            .await;
        let account_balances_full = dummy_balances(100, 100);

        // add all transactions to the container
        pending_txs
            .add(ttx_bundleable_general.clone(), 0, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_unbundleable_general.clone(), 0, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_bundleable_sudo.clone(), 0, &account_balances_full)
            .unwrap();
        pending_txs
            .add(ttx_unbundleable_sudo.clone(), 0, &account_balances_full)
            .unwrap();

        // get the builder queue
        // note: the account nonces are set to zero when not initialized in the mock state
        let builder_queue = pending_txs.builder_queue();

        // check that the transactions are in the expected order
        let first_tx_id = builder_queue[0].id();
        assert_eq!(
            first_tx_id,
            ttx_bundleable_general.checked_tx.id(),
            "expected bundleable general transaction to be first"
        );

        let second_tx_id = builder_queue[1].id();
        assert_eq!(
            second_tx_id,
            ttx_unbundleable_general.checked_tx.id(),
            "expected unbundleable general transaction to be second"
        );

        let third_tx_id = builder_queue[2].id();
        assert_eq!(
            third_tx_id,
            ttx_bundleable_sudo.checked_tx.id(),
            "expected bundleable sudo transaction to be third"
        );

        let fourth_tx_id = builder_queue[3].id();
        assert_eq!(
            fourth_tx_id,
            ttx_unbundleable_sudo.checked_tx.id(),
            "expected unbundleable sudo transaction to be last"
        );
    }

    #[tokio::test]
    async fn parked_transactions_size_limit_works() {
        let fixture = Fixture::default_initialized().await;
        let mut parked_txs = ParkedTransactions::<MAX_PARKED_TXS_PER_ACCOUNT>::new(TX_TTL, 1);

        // transactions to add to account
        let ttx_1 = MockTTXBuilder::new(&fixture).nonce(1).build().await;
        let ttx_2 = MockTTXBuilder::new(&fixture).nonce(2).build().await;
        let account_balances_full = dummy_balances(100, 100);

        // under limit okay
        parked_txs
            .add(ttx_1.clone(), 1, &account_balances_full)
            .unwrap();

        // growing past limit causes error
        assert_eq!(
            parked_txs
                .add(ttx_2.clone(), 0, &account_balances_full)
                .unwrap_err(),
            InsertionError::ParkedSizeLimit,
            "size limit should be enforced"
        );

        // removing transactions makes space for new ones
        parked_txs.remove(ttx_1.checked_tx).unwrap();
        // adding should now be okay
        parked_txs.add(ttx_2, 0, &account_balances_full).unwrap();
    }
}
