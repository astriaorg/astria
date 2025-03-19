use std::{
    collections::{
        HashMap,
        HashSet,
        VecDeque,
    },
    num::NonZeroUsize,
    sync::Arc,
};

use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
    },
    protocol::transaction::v1::{
        Transaction,
        TransactionStatus,
    },
};
use astria_eyre::eyre::Result;
use tokio::time::Duration;
use tracing::{
    error,
    instrument,
    warn,
    Level,
};

use super::transactions_container::{
    ContainedTxs,
    ParkedTransactions,
    PendingTransactions,
    TimemarkedTransaction,
    TransactionsContainer as _,
};
pub(crate) use super::{
    mempool_state::get_account_balances,
    transactions_container::InsertionError,
};
use crate::{
    accounts,
    Metrics,
};

/// How long transactions are considered valid in the mempool.
const TX_TTL: Duration = Duration::from_secs(240);
/// Max number of parked transactions allowed per account.
const MAX_PARKED_TXS_PER_ACCOUNT: usize = 15;
/// Max number of transactions to keep in the removal cache. Should be larger than the max number of
/// transactions allowed in the cometBFT mempool.
pub(crate) const REMOVAL_CACHE_SIZE: usize = 50_000;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    NonceStale,
    LowerNonceInvalidated,
    FailedPrepareProposal(String),
}

impl std::fmt::Display for RemovalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemovalReason::Expired => {
                write!(f, "transaction expired after {} seconds", TX_TTL.as_secs())
            }
            RemovalReason::NonceStale => write!(f, "transaction nonce is lower than current nonce"),
            RemovalReason::LowerNonceInvalidated => write!(
                f,
                "previous transaction was not executed, to this transaction's nonce has become \
                 invalid"
            ),
            RemovalReason::FailedPrepareProposal(reason) => {
                write!(f, "failed `prepare_proposal`: {reason}")
            }
        }
    }
}

/// `RemovalCache` is used to signal to `CometBFT` that a
/// transaction can be removed from the `CometBFT` mempool.
///
/// This is useful for when a transaction fails execution or when
/// a transaction is invalidated due to mempool removal policies.
#[derive(Clone)]
pub(crate) struct RemovalCache {
    cache: HashMap<[u8; 32], RemovalReason>,
    remove_queue: VecDeque<[u8; 32]>,
    max_size: NonZeroUsize,
}

impl RemovalCache {
    pub(super) fn new(max_size: NonZeroUsize) -> Self {
        Self {
            cache: HashMap::new(),
            remove_queue: VecDeque::with_capacity(max_size.into()),
            max_size,
        }
    }

    /// Returns Some(RemovalReason) if the transaction is cached and
    /// removes the entry from the cache if present.
    pub(super) fn remove(&mut self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.cache.remove(&tx_hash)
    }

    /// Adds the transaction to the cache, will preserve the original
    /// `RemovalReason` if already in the cache.
    pub(super) fn add(&mut self, tx_hash: [u8; 32], reason: RemovalReason) {
        if self.cache.contains_key(&tx_hash) {
            return;
        };

        if self.remove_queue.len() == usize::from(self.max_size) {
            // This should not happen if `REMOVAL_CACHE_SIZE` is >= CometBFT's configured mempool
            // size.
            //
            // Make space for the new transaction by removing the oldest transaction.
            let removed_tx = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            warn!(
                tx_hash = %telemetry::display::hex(&removed_tx),
                removal_cache_size = REMOVAL_CACHE_SIZE,
                "popped transaction from appside mempool removal cache, CometBFT will not remove \
                this transaction from its mempool - removal cache size possibly too low"
            );
            // Remove transaction from cache if it is present.
            self.cache.remove(&removed_tx);
        }
        self.remove_queue.push_back(tx_hash);
        self.cache.insert(tx_hash, reason);
    }

    /// Returns the removal reason for the transaction if it is present in the cache.
    fn get(&self, tx_hash: [u8; 32]) -> Option<&RemovalReason> {
        self.cache.get(&tx_hash)
    }
}

#[derive(Clone)]
pub(super) struct MempoolInner {
    pub(super) pending: PendingTransactions,
    pub(super) parked: ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT>,
    pub(super) comet_bft_removal_cache: RemovalCache,
    pub(super) contained_txs: ContainedTxs,
    pub(super) metrics: &'static Metrics,
}

impl MempoolInner {
    #[must_use]
    pub(super) fn new(metrics: &'static Metrics, parked_max_tx_count: usize) -> Self {
        Self {
            pending: PendingTransactions::new(TX_TTL),
            parked: ParkedTransactions::new(TX_TTL, parked_max_tx_count),
            comet_bft_removal_cache: RemovalCache::new(
                NonZeroUsize::try_from(REMOVAL_CACHE_SIZE)
                    .expect("Removal cache cannot be zero sized"),
            ),
            contained_txs: ContainedTxs::new(metrics),
            metrics,
        }
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all, fields(tx_hash = %tx.id(), current_account_nonce), err(level = Level::DEBUG))]
    pub(super) async fn insert(
        &mut self,
        tx: Arc<Transaction>,
        current_account_nonce: u32,
        current_account_balances: HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> Result<(), InsertionError> {
        let timemarked_tx = TimemarkedTransaction::new(tx, transaction_cost);
        let id = timemarked_tx.id();

        // try insert into pending
        match self.pending.add(
            timemarked_tx.clone(),
            current_account_nonce,
            &current_account_balances,
        ) {
            Err(InsertionError::NonceGap | InsertionError::AccountBalanceTooLow) => {
                // try to add to parked queue
                match self.parked.add(
                    timemarked_tx,
                    current_account_nonce,
                    &current_account_balances,
                ) {
                    Ok(()) => {
                        // log current size of parked
                        self.metrics
                            .set_transactions_in_mempool_parked(self.parked.len());

                        // track in contained txs
                        self.contained_txs.add(id);
                        Ok(())
                    }
                    Err(err) => Err(err),
                }
            }
            error @ Err(
                InsertionError::AlreadyPresent
                | InsertionError::NonceTooLow
                | InsertionError::NonceTaken
                | InsertionError::AccountSizeLimit
                | InsertionError::ParkedSizeLimit,
            ) => error,
            Ok(()) => {
                // check parked for txs able to be promoted
                let to_promote = self.parked.find_promotables(
                    timemarked_tx.address(),
                    timemarked_tx
                        .nonce()
                        .checked_add(1)
                        .expect("failed to increment nonce in promotion"),
                    &self.pending.subtract_contained_costs(
                        timemarked_tx.address(),
                        current_account_balances.clone(),
                    ),
                );
                // promote the transactions
                for ttx in to_promote {
                    let tx_id = ttx.id();
                    if let Err(error) =
                        self.pending
                            .add(ttx, current_account_nonce, &current_account_balances)
                    {
                        self.contained_txs.remove(timemarked_tx.id());
                        error!(
                            current_account_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to promote transaction during insertion"
                        );
                    }
                }

                // track in contained txs
                self.contained_txs.add(timemarked_tx.id());

                Ok(())
            }
        }
    }

    /// Removes the target transaction and all transactions for associated account with higher
    /// nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and not executed
    /// transactions. Executed transactions will be removed in the `run_maintenance()` function.
    #[instrument(skip_all, fields(tx_hash = %signed_tx.id()))]
    pub(crate) async fn remove_tx_invalid(
        &mut self,
        signed_tx: Arc<Transaction>,
        reason: RemovalReason,
    ) {
        let tx_hash = signed_tx.id().get();
        let address = *signed_tx.verification_key().address_bytes();

        // Try to remove from pending.
        let removed_txs = match self.pending.remove(signed_tx) {
            Ok(mut removed_txs) => {
                // Remove all of parked.
                removed_txs.append(&mut self.parked.clear_account(&address));
                removed_txs
            }
            Err(signed_tx) => {
                // Not found in pending, try to remove from parked and if not found, just return.
                match self.parked.remove(signed_tx) {
                    Ok(removed_txs) => removed_txs,
                    Err(_) => return,
                }
            }
        };

        // Add the original tx first to preserve its reason for removal. The second
        // attempt to add it inside the loop below will be a no-op.
        self.comet_bft_removal_cache.add(tx_hash, reason);
        for removed_tx in removed_txs {
            self.contained_txs.remove(removed_tx);
            self.comet_bft_removal_cache
                .add(removed_tx, RemovalReason::LowerNonceInvalidated);
        }
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces or are expired. Will also shift transation between pending and
    /// parked to relfect changes in account balances.
    ///
    /// All removed transactions are added to the CometBFT removal cache to aid with CometBFT
    /// mempool maintenance.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<S: accounts::StateReadExt>(
        &mut self,
        state: &S,
        recost: bool,
    ) {
        let mut removed_txs = Vec::<([u8; 32], RemovalReason)>::new();

        // To clean we need to:
        // 1.) remove stale and expired transactions
        // 2.) recost remaining transactions if needed
        // 3.) check if we have transactions in pending which need to be demoted due
        //     to balance decreases
        // 4.) if there were no demotions, check if parked has transactions we can
        //     promote

        let addresses: HashSet<[u8; 20]> = self
            .pending
            .addresses()
            .chain(self.parked.addresses())
            .copied()
            .collect();

        // TODO: Make this concurrent, all account state is separate with IO bound disk reads.
        for address in &addresses {
            // get current account state
            let current_nonce = match state.get_account_nonce(address).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(&address),
                        "failed to fetch account nonce when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };
            let current_balances = match get_account_balances(state, address).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(address),
                        "failed to fetch account balances when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };

            // clean pending and parked of stale and expired
            removed_txs.extend(
                self.pending
                    .clean_account_stale_expired(address, current_nonce),
            );
            if recost {
                self.pending.recost_transactions(address, state).await;
            }

            removed_txs.extend(
                self.parked
                    .clean_account_stale_expired(address, current_nonce),
            );
            if recost {
                self.parked.recost_transactions(address, state).await;
            }

            // get transactions to demote from pending
            let demotion_txs = self.pending.find_demotables(address, &current_balances);

            if demotion_txs.is_empty() {
                // nothing to demote, check for transactions to promote
                let pending_nonce = self
                    .pending
                    .pending_nonce(address)
                    .map_or(current_nonce, |nonce| nonce);

                let remaining_balances = self
                    .pending
                    .subtract_contained_costs(address, current_balances.clone());
                let promotion_txs =
                    self.parked
                        .find_promotables(address, pending_nonce, &remaining_balances);

                for tx in promotion_txs {
                    let tx_id = tx.id();
                    if let Err(error) = self.pending.add(tx, current_nonce, &current_balances) {
                        // NOTE: this shouldn't happen. Promotions should never fail. This also
                        // means grabbing the lock inside the loop is more
                        // performant.
                        self.contained_txs.remove(tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address),
                            current_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to promote transaction during maintenance"
                        );
                    }
                }
            } else {
                // add demoted transactions to parked
                for tx in demotion_txs {
                    let tx_id = tx.id();
                    if let Err(error) = self.parked.add(tx, current_nonce, &current_balances) {
                        // NOTE: this shouldn't happen normally but could on the edge case of
                        // the parked queue being full for the account or globally.
                        // Grabbing the lock inside the loop should be more performant.
                        self.contained_txs.remove(tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address),
                            current_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to demote transaction during maintenance"
                        );
                    }
                }
            }
        }

        // add to removal cache for cometbft and remove from the tracked set
        for (tx_hash, reason) in removed_txs {
            self.comet_bft_removal_cache.add(tx_hash, reason);
            self.contained_txs.remove(tx_hash);
        }
    }

    /// Returns a given transaction's status, as well as an optional reason for removal if the
    /// transaction is in the CometBFT removal cache.
    pub(in crate::mempool) fn get_transaction_status(
        &self,
        tx_id: &TransactionId,
    ) -> (TransactionStatus, Option<String>) {
        if self.contained_txs.contains(tx_id.as_bytes()) {
            if self.pending.contains(tx_id) {
                return (TransactionStatus::Pending, None);
            }
            if self.parked.contains(tx_id) {
                return (TransactionStatus::Parked, None);
            }
        }
        if let Some(reason) = self.comet_bft_removal_cache.get(tx_id.get()) {
            return (TransactionStatus::RemovalCache, Some(reason.to_string()));
        }
        (TransactionStatus::NotFound, None)
    }
}
