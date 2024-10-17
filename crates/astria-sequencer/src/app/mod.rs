mod action_handler;
#[cfg(any(test, feature = "benchmark"))]
pub(crate) mod benchmark_and_test_utils;
#[cfg(feature = "benchmark")]
mod benchmarks;
mod state_ext;
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod test_utils;
#[cfg(test)]
mod tests_app;
#[cfg(test)]
mod tests_block_ordering;
#[cfg(test)]
mod tests_breaking_changes;
#[cfg(test)]
mod tests_execute_transaction;

use std::{
    collections::VecDeque,
    sync::Arc,
};

use astria_core::{
    generated::protocol::transaction::v1 as raw,
    protocol::{
        abci::AbciErrorCode,
        genesis::v1::GenesisAppState,
        transaction::v1::{
            action::{
                group::Group,
                ValidatorUpdate,
            },
            Action,
            Transaction,
        },
    },
    sequencerblock::v1::block::SequencerBlock,
    Protobuf as _,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    ArcStateDeltaExt as _,
    StagedWriteBatch,
    StateDelta,
    StateRead,
    StateWrite,
};
use prost::Message as _;
use sha2::{
    Digest as _,
    Sha256,
};
use telemetry::display::json;
use tendermint::{
    abci::{
        self,
        types::ExecTxResult,
        Code,
        Event,
    },
    account,
    block::Header,
    AppHash,
    Hash,
};
use tracing::{
    debug,
    info,
    instrument,
};

pub(crate) use self::{
    action_handler::ActionHandler,
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
};
use crate::{
    accounts::{
        component::AccountsComponent,
        StateWriteExt as _,
    },
    address::StateWriteExt as _,
    assets::StateWriteExt as _,
    authority::{
        component::{
            AuthorityComponent,
            AuthorityComponentAppState,
        },
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    component::Component as _,
    fees::{
        component::FeesComponent,
        StateReadExt as _,
    },
    grpc::StateWriteExt as _,
    ibc::component::IbcComponent,
    mempool::{
        Mempool,
        RemovalReason,
    },
    metrics::Metrics,
    proposal::{
        block_size_constraints::BlockSizeConstraints,
        commitment::{
            generate_rollup_datas_commitment,
            GeneratedCommitments,
        },
    },
    storage::{
        Snapshot,
        Storage,
    },
    transaction::InvalidNonce,
};

// ephemeral store key for the cache of results of executing of transactions in `prepare_proposal`.
// cleared in `process_proposal` if we're the proposer.
const EXECUTION_RESULTS_KEY: &str = "execution_results";

// ephemeral store key for the cache of results of executing of transactions in `process_proposal`.
// cleared at the end of the block.
const POST_TRANSACTION_EXECUTION_RESULT_KEY: &str = "post_transaction_execution_result";

/// The inter-block state being written to by the application.
type InterBlockState = Arc<StateDelta<Snapshot>>;

/// This is used to identify a proposal constructed by the app instance
/// in `prepare_proposal` during a `process_proposal` call.
///
/// The fields are not exhaustive, in most instances just the validator address
/// is adequate. When running a third party signer such as horcrux however it is
/// possible that multiple nodes are preparing proposals as the same validator
/// address, in these instances the timestamp is used as a unique identifier for
/// the proposal from that node. This is not a perfect solution, but it only
/// impacts sentry nodes does not halt the network and is cheaper computationally
/// than an exhaustive comparison.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ProposalFingerprint {
    validator_address: account::Id,
    timestamp: tendermint::Time,
}

impl From<abci::request::PrepareProposal> for ProposalFingerprint {
    fn from(proposal: abci::request::PrepareProposal) -> Self {
        Self {
            validator_address: proposal.proposer_address,
            timestamp: proposal.time,
        }
    }
}

impl From<abci::request::ProcessProposal> for ProposalFingerprint {
    fn from(proposal: abci::request::ProcessProposal) -> Self {
        Self {
            validator_address: proposal.proposer_address,
            timestamp: proposal.time,
        }
    }
}

/// The Sequencer application, written as a bundle of [`Component`]s.
///
/// Note: this is called `App` because this is a Tendermint ABCI application,
/// and implements the state transition logic of the chain.
///
/// See also the [Penumbra reference] implementation.
///
/// [Penumbra reference]: https://github.com/penumbra-zone/penumbra/blob/9cc2c644e05c61d21fdc7b507b96016ba6b9a935/app/src/app/mod.rs#L42
pub(crate) struct App {
    state_delta: InterBlockState,

    // The mempool of the application.
    //
    // Transactions are pulled from this mempool during `prepare_proposal`.
    mempool: Mempool,

    // TODO(https://github.com/astriaorg/astria/issues/1660): The executed_proposal_fingerprint and
    // executed_proposal_hash fields should be stored in the ephemeral storage instead of on the
    // app struct, to avoid any issues with forgetting to reset them.

    // An identifier for a given proposal constructed by this app.
    //
    // Used to avoid executing a block in both `prepare_proposal` and `process_proposal`. It
    // is set in `prepare_proposal` from information sent in from cometbft and can potentially
    // change round-to-round. In `process_proposal` we check if we prepared the proposal, and
    // if so, we clear the value, and we skip re-execution of the block's transactions to avoid
    // failures caused by re-execution.
    executed_proposal_fingerprint: Option<ProposalFingerprint>,

    // This is set to the executed hash of the proposal during `process_proposal`
    //
    // If it does not match the hash given during `begin_block`, then we clear and
    // reset the execution results cache + state delta. Transactions are re-executed.
    // If it does match, we utilize cached results to reduce computation.
    //
    // Resets to default hash at the beginning of `prepare_proposal`, and `process_proposal` if
    // `prepare_proposal` was not called.
    executed_proposal_hash: Hash,

    // This is set when a `FeeChange` or `FeeAssetChange` action is seen in a block to flag
    // to the mempool to recost all transactions.
    recost_mempool: bool,

    // the current `StagedWriteBatch` which contains the rocksdb write batch
    // of the current block being executed, created from the state delta,
    // and set after `finalize_block`.
    // this is committed to the state when `commit` is called, and set to `None`.
    write_batch: Option<StagedWriteBatch>,

    // the currently committed `AppHash` of the application state.
    // set whenever `commit` is called.
    #[expect(
        clippy::struct_field_names,
        reason = "we need to be specific as to what hash this is"
    )]
    app_hash: AppHash,

    metrics: &'static Metrics,
}

impl App {
    pub(crate) async fn new(
        snapshot: Snapshot,
        mempool: Mempool,
        metrics: &'static Metrics,
    ) -> Result<Self> {
        debug!("initializing App instance");

        let app_hash: AppHash = snapshot
            .root_hash()
            .await
            .wrap_err("failed to get current root hash")?
            .0
            .to_vec()
            .try_into()
            .expect("root hash conversion must succeed; should be 32 bytes");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state_delta = Arc::new(snapshot.new_delta());

        Ok(Self {
            state_delta,
            mempool,
            executed_proposal_fingerprint: None,
            executed_proposal_hash: Hash::default(),
            recost_mempool: false,
            write_batch: None,
            app_hash,
            metrics,
        })
    }

    #[instrument(name = "App:init_chain", skip_all)]
    pub(crate) async fn init_chain(
        &mut self,
        storage: Storage,
        genesis_state: GenesisAppState,
        genesis_validators: Vec<ValidatorUpdate>,
        chain_id: String,
    ) -> Result<AppHash> {
        let mut delta_delta = self
            .state_delta
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        delta_delta
            .put_base_prefix(genesis_state.address_prefixes().base().to_string())
            .wrap_err("failed to write base prefix to state")?;
        delta_delta
            .put_ibc_compat_prefix(genesis_state.address_prefixes().ibc_compat().to_string())
            .wrap_err("failed to write ibc-compat prefix to state")?;

        if let Some(native_asset) = genesis_state.native_asset_base_denomination() {
            delta_delta
                .put_native_asset(native_asset.clone())
                .wrap_err("failed to write native asset to state")?;
            delta_delta
                .put_ibc_asset(native_asset.clone())
                .wrap_err("failed to commit native asset as ibc asset to state")?;
        }

        delta_delta
            .put_chain_id_and_revision_number(chain_id.try_into().context("invalid chain ID")?)
            .wrap_err("failed to write chain id to state")?;
        delta_delta
            .put_block_height(0)
            .wrap_err("failed to write block height to state")?;

        // call init_chain on all components
        FeesComponent::init_chain(&mut delta_delta, &genesis_state)
            .await
            .wrap_err("init_chain failed on FeesComponent")?;
        AccountsComponent::init_chain(&mut delta_delta, &genesis_state)
            .await
            .wrap_err("init_chain failed on AccountsComponent")?;
        AuthorityComponent::init_chain(
            &mut delta_delta,
            &AuthorityComponentAppState {
                authority_sudo_address: *genesis_state.authority_sudo_address(),
                genesis_validators,
            },
        )
        .await
        .wrap_err("init_chain failed on AuthorityComponent")?;
        IbcComponent::init_chain(&mut delta_delta, &genesis_state)
            .await
            .wrap_err("init_chain failed on IbcComponent")?;

        delta_delta.apply();

        let app_hash = self
            .prepare_commit(storage)
            .await
            .wrap_err("failed to prepare commit")?;
        debug!(app_hash = %telemetry::display::base64(&app_hash), "init_chain completed");
        Ok(app_hash)
    }

    fn update_state_for_new_round(&mut self, storage: &Storage) {
        // reset app state to latest committed state, in case of a round not being committed
        // but `self.state` was changed due to executing the previous round's data.
        //
        // if the previous round was committed, then the state stays the same.
        //
        // this also clears the ephemeral storage.
        self.state_delta = Arc::new(storage.new_delta_of_latest_snapshot());

        // clear the cached executed proposal hash
        self.executed_proposal_hash = Hash::default();
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions.
    ///
    /// This is required so that a rollup can easily verify that the transactions it
    /// receives are correct (ie. we actually included in a sequencer block, and none
    /// are missing)
    /// It puts this special "commitment" as the first transaction in a block.
    /// When other validators receive the block, they know the first transaction is
    /// supposed to be the commitment, and verifies that is it correct.
    #[instrument(name = "App::prepare_proposal", skip_all)]
    pub(crate) async fn prepare_proposal(
        &mut self,
        prepare_proposal: abci::request::PrepareProposal,
        storage: Storage,
    ) -> Result<abci::response::PrepareProposal> {
        self.executed_proposal_fingerprint = Some(prepare_proposal.clone().into());
        self.update_state_for_new_round(&storage);

        let mut block_size_constraints = BlockSizeConstraints::new(
            usize::try_from(prepare_proposal.max_tx_bytes)
                .wrap_err("failed to convert max_tx_bytes to usize")?,
        )
        .wrap_err("failed to create block size constraints")?;

        let block_data = BlockData {
            misbehavior: prepare_proposal.misbehavior,
            height: prepare_proposal.height,
            time: prepare_proposal.time,
            next_validators_hash: prepare_proposal.next_validators_hash,
            proposer_address: prepare_proposal.proposer_address,
        };

        self.pre_execute_transactions(block_data)
            .await
            .wrap_err("failed to prepare for executing block")?;

        // ignore the txs passed by cometbft in favour of our app-side mempool
        let (included_tx_bytes, signed_txs_included) = self
            .execute_transactions_prepare_proposal(&mut block_size_constraints)
            .await
            .wrap_err("failed to execute transactions")?;
        self.metrics
            .record_proposal_transactions(signed_txs_included.len());

        let deposits = self.state_delta.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        // generate commitment to sequence::Actions and deposits and commitment to the rollup IDs
        // included in the block
        let res = generate_rollup_datas_commitment(&signed_txs_included, deposits);
        let txs = res.into_transactions(included_tx_bytes);
        Ok(abci::response::PrepareProposal {
            txs,
        })
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions
    /// and ensures it matches the commitment created by the proposer, which
    /// should be the first transaction in the block.
    #[instrument(name = "App::process_proposal", skip_all)]
    pub(crate) async fn process_proposal(
        &mut self,
        process_proposal: abci::request::ProcessProposal,
        storage: Storage,
    ) -> Result<()> {
        // if we proposed this block (ie. prepare_proposal was called directly before this), then
        // we skip execution for this `process_proposal` call.
        //
        // if we didn't propose this block, `self.validator_address` will be None or a different
        // value, so we will execute  block as normal.
        if let Some(constructed_id) = self.executed_proposal_fingerprint {
            let proposal_id = process_proposal.clone().into();
            if constructed_id == proposal_id {
                debug!("skipping process_proposal as we are the proposer for this block");
                self.executed_proposal_fingerprint = None;
                self.executed_proposal_hash = process_proposal.hash;

                // if we're the proposer, we should have the execution results from
                // `prepare_proposal`. run the post-tx-execution hook to generate the
                // `SequencerBlock` and to set `self.finalize_block`.
                //
                // we can't run this in `prepare_proposal` as we don't know the block hash there.
                let Some(tx_results) = self.state_delta.object_get(EXECUTION_RESULTS_KEY) else {
                    bail!("execution results must be present after executing transactions")
                };

                self.post_execute_transactions(
                    process_proposal.hash,
                    process_proposal.height,
                    process_proposal.time,
                    process_proposal.proposer_address,
                    process_proposal.txs,
                    tx_results,
                )
                .await
                .wrap_err("failed to run post execute transactions handler")?;

                return Ok(());
            }
            self.metrics.increment_process_proposal_skipped_proposal();
            debug!(
                "our validator address was set but we're not the proposer, so our previous \
                 proposal was skipped, executing block"
            );
            self.executed_proposal_fingerprint = None;
        }

        self.update_state_for_new_round(&storage);

        let mut txs = VecDeque::from(process_proposal.txs.clone());
        let received_rollup_datas_root: [u8; 32] = txs
            .pop_front()
            .ok_or_eyre("no transaction commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| eyre!("transaction commitment must be 32 bytes"))?;

        let received_rollup_ids_root: [u8; 32] = txs
            .pop_front()
            .ok_or_eyre("no chain IDs commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| eyre!("chain IDs commitment must be 32 bytes"))?;

        let expected_txs_len = txs.len();

        let block_data = BlockData {
            misbehavior: process_proposal.misbehavior,
            height: process_proposal.height,
            time: process_proposal.time,
            next_validators_hash: process_proposal.next_validators_hash,
            proposer_address: process_proposal.proposer_address,
        };

        self.pre_execute_transactions(block_data)
            .await
            .wrap_err("failed to prepare for executing block")?;

        // we don't care about the cometbft max_tx_bytes here, as cometbft would have
        // rejected the proposal if it was too large.
        // however, we should still validate the other constraints, namely
        // the max sequenced data bytes.
        let mut block_size_constraints = BlockSizeConstraints::new_unlimited_cometbft();

        // deserialize txs into `Transaction`s;
        // this does not error if any txs fail to be deserialized, but the `execution_results.len()`
        // check below ensures that all txs in the proposal are deserializable (and
        // executable).
        let signed_txs = txs
            .into_iter()
            .filter_map(|bytes| signed_transaction_from_bytes(bytes.as_ref()).ok())
            .collect::<Vec<_>>();

        let tx_results = self
            .execute_transactions_process_proposal(signed_txs.clone(), &mut block_size_constraints)
            .await
            .wrap_err("failed to execute transactions")?;

        // all txs in the proposal should be deserializable and executable
        // if any txs were not deserializeable or executable, they would not have been
        // added to the `tx_results` list, thus the length of `txs_to_include`
        // will be shorter than that of `tx_results`.
        ensure!(
            tx_results.len() == expected_txs_len,
            "transactions to be included do not match expected",
        );
        self.metrics.record_proposal_transactions(signed_txs.len());

        let deposits = self.state_delta.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        let GeneratedCommitments {
            rollup_datas_root: expected_rollup_datas_root,
            rollup_ids_root: expected_rollup_ids_root,
        } = generate_rollup_datas_commitment(&signed_txs, deposits);
        ensure!(
            received_rollup_datas_root == expected_rollup_datas_root,
            "transaction commitment does not match expected",
        );

        ensure!(
            received_rollup_ids_root == expected_rollup_ids_root,
            "chain IDs commitment does not match expected",
        );

        self.executed_proposal_hash = process_proposal.hash;
        self.post_execute_transactions(
            process_proposal.hash,
            process_proposal.height,
            process_proposal.time,
            process_proposal.proposer_address,
            process_proposal.txs,
            tx_results,
        )
        .await
        .wrap_err("failed to run post execute transactions handler")?;

        Ok(())
    }

    /// Executes transactions from the app's mempool until the block is full,
    /// writing to the app's `StateDelta`.
    ///
    /// The result of execution of every transaction which is successful
    /// is stored in ephemeral storage for usage in `process_proposal`.
    ///
    /// Returns the transactions which were successfully executed
    /// in both their [`Transaction`] and raw bytes form.
    ///
    /// Unlike the usual flow of an ABCI application, this is called during
    /// the proposal phase, ie. `prepare_proposal`.
    ///
    /// This is because we disallow transactions that fail execution to be included
    /// in a block's transaction data, as this would allow `sequence::Action`s to be
    /// included for free. Instead, we execute transactions during the proposal phase,
    /// and only include them in the block if they succeed.
    ///
    /// As a result, all transactions in a sequencer block are guaranteed to execute
    /// successfully.
    #[instrument(name = "App::execute_transactions_prepare_proposal", skip_all)]
    async fn execute_transactions_prepare_proposal(
        &mut self,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<(Vec<bytes::Bytes>, Vec<Transaction>)> {
        let mempool_len = self.mempool.len().await;
        debug!(mempool_len, "executing transactions from mempool");

        let mut validated_txs: Vec<bytes::Bytes> = Vec::new();
        let mut included_signed_txs = Vec::new();
        let mut failed_tx_count: usize = 0;
        let mut execution_results = Vec::new();
        let mut excluded_txs: usize = 0;
        let mut current_tx_group = Group::BundleableGeneral;

        // get copy of transactions to execute from mempool
        let pending_txs = self
            .mempool
            .builder_queue(&self.state_delta)
            .await
            .expect("failed to fetch pending transactions");

        let mut unused_count = pending_txs.len();
        for (tx_hash, tx) in pending_txs {
            unused_count = unused_count.saturating_sub(1);
            let tx_hash_base64 = telemetry::display::base64(&tx_hash).to_string();
            let bytes = tx.to_raw().encode_to_vec();
            let tx_len = bytes.len();
            info!(transaction_hash = %tx_hash_base64, "executing transaction");

            // don't include tx if it would make the cometBFT block too large
            if !block_size_constraints.cometbft_has_space(tx_len) {
                self.metrics
                    .increment_prepare_proposal_excluded_transactions_cometbft_space();
                debug!(
                    transaction_hash = %tx_hash_base64,
                    block_size_constraints = %json(&block_size_constraints),
                    tx_data_bytes = tx_len,
                    "excluding remaining transactions: max cometBFT data limit reached"
                );
                excluded_txs = excluded_txs.saturating_add(1);

                // break from loop, as the block is full
                break;
            }

            // check if tx's sequence data will fit into sequence block
            let tx_sequence_data_bytes = tx
                .unsigned_transaction()
                .actions()
                .iter()
                .filter_map(Action::as_rollup_data_submission)
                .fold(0usize, |acc, seq| acc.saturating_add(seq.data.len()));

            if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
                self.metrics
                    .increment_prepare_proposal_excluded_transactions_sequencer_space();
                debug!(
                    transaction_hash = %tx_hash_base64,
                    block_size_constraints = %json(&block_size_constraints),
                    tx_data_bytes = tx_sequence_data_bytes,
                    "excluding transaction: max block sequenced data limit reached"
                );
                excluded_txs = excluded_txs.saturating_add(1);

                // continue as there might be non-sequence txs that can fit
                continue;
            }

            // ensure transaction's group is less than or equal to current action group
            let tx_group = tx.group();
            if tx_group > current_tx_group {
                debug!(
                    transaction_hash = %tx_hash_base64,
                    block_size_constraints = %json(&block_size_constraints),
                    "excluding transaction: group is higher priority than previously included transactions"
                );
                excluded_txs = excluded_txs.saturating_add(1);

                // note: we don't remove the tx from mempool as it may be valid in the future
                continue;
            }

            // execute tx and store in `execution_results` list on success
            match self.execute_transaction(tx.clone()).await {
                Ok(events) => {
                    execution_results.push(ExecTxResult {
                        events,
                        ..Default::default()
                    });
                    block_size_constraints
                        .sequencer_checked_add(tx_sequence_data_bytes)
                        .wrap_err("error growing sequencer block size")?;
                    block_size_constraints
                        .cometbft_checked_add(tx_len)
                        .wrap_err("error growing cometBFT block size")?;
                    validated_txs.push(bytes.into());
                    included_signed_txs.push((*tx).clone());
                }
                Err(e) => {
                    self.metrics
                        .increment_prepare_proposal_excluded_transactions_failed_execution();
                    debug!(
                        transaction_hash = %tx_hash_base64,
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                        "failed to execute transaction, not including in block"
                    );

                    if e.downcast_ref::<InvalidNonce>().is_some() {
                        // we don't remove the tx from mempool if it failed to execute
                        // due to an invalid nonce, as it may be valid in the future.
                        // if it's invalid due to the nonce being too low, it'll be
                        // removed from the mempool in `update_mempool_after_finalization`.
                        //
                        // this is important for possible out-of-order transaction
                        // groups fed into prepare_proposal. a transaction with a higher
                        // nonce might be in a higher priority group than a transaction
                        // from the same account wiht a lower nonce. this higher nonce
                        // could execute in the next block fine.
                    } else {
                        failed_tx_count = failed_tx_count.saturating_add(1);

                        // remove the failing transaction from the mempool
                        //
                        // this will remove any transactions from the same sender
                        // as well, as the dependent nonces will not be able
                        // to execute
                        self.mempool
                            .remove_tx_invalid(
                                tx,
                                RemovalReason::FailedPrepareProposal(e.to_string()),
                            )
                            .await;
                    }
                }
            }

            // update current action group to tx's action group
            current_tx_group = tx_group;
        }

        if failed_tx_count > 0 {
            info!(
                failed_tx_count = failed_tx_count,
                included_tx_count = validated_txs.len(),
                "excluded transactions from block due to execution failure"
            );
        }
        self.metrics.set_prepare_proposal_excluded_transactions(
            excluded_txs.saturating_add(failed_tx_count),
        );

        debug!("{unused_count} leftover pending transactions");
        self.metrics
            .set_transactions_in_mempool_total(self.mempool.len().await);

        // XXX: we need to unwrap the app's state arc to write
        // to the ephemeral store.
        // this is okay as we should have the only reference to the state
        // at this point.
        let mut delta_delta = Arc::try_begin_transaction(&mut self.state_delta)
            .expect("state Arc should not be referenced elsewhere");
        delta_delta.object_put(EXECUTION_RESULTS_KEY, execution_results);
        let _ = delta_delta.apply();

        Ok((validated_txs, included_signed_txs))
    }

    /// Executes the given transactions, writing to the app's `StateDelta`.
    ///
    /// Unlike the usual flow of an ABCI application, this is called during
    /// the proposal phase, ie. `process_proposal`.
    ///
    /// This is because we disallow transactions that fail execution to be included
    /// in a block's transaction data, as this would allow `sequence::Action`s to be
    /// included for free. Instead, we execute transactions during the proposal phase,
    /// and only include them in the block if they succeed.
    ///
    /// As a result, all transactions in a sequencer block are guaranteed to execute
    /// successfully.
    #[instrument(name = "App::execute_transactions_process_proposal", skip_all)]
    async fn execute_transactions_process_proposal(
        &mut self,
        txs: Vec<Transaction>,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<Vec<ExecTxResult>> {
        let mut execution_results = Vec::new();
        let mut current_tx_group = Group::BundleableGeneral;

        for tx in txs {
            let bytes = tx.to_raw().encode_to_vec();
            let tx_hash = Sha256::digest(&bytes);
            let tx_len = bytes.len();

            // check if tx's sequence data will fit into sequence block
            let tx_sequence_data_bytes = tx
                .unsigned_transaction()
                .actions()
                .iter()
                .filter_map(Action::as_rollup_data_submission)
                .fold(0usize, |acc, seq| acc.saturating_add(seq.data.len()));

            if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    block_size_constraints = %json(&block_size_constraints),
                    tx_data_bytes = tx_sequence_data_bytes,
                    "transaction error: max block sequenced data limit passed"
                );
                bail!("max block sequenced data limit passed");
            }

            // ensure transaction's group is less than or equal to current action group
            let tx_group = tx.group();
            if tx_group > current_tx_group {
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    "transaction error: block has incorrect transaction group ordering"
                );
                bail!("transactions have incorrect transaction group ordering");
            }

            // execute tx and store in `execution_results` list on success
            match self.execute_transaction(Arc::new(tx.clone())).await {
                Ok(events) => {
                    execution_results.push(ExecTxResult {
                        events,
                        ..Default::default()
                    });
                    block_size_constraints
                        .sequencer_checked_add(tx_sequence_data_bytes)
                        .wrap_err("error growing sequencer block size")?;
                    block_size_constraints
                        .cometbft_checked_add(tx_len)
                        .wrap_err("error growing cometBFT block size")?;
                }
                Err(e) => {
                    debug!(
                        transaction_hash = %telemetry::display::base64(&tx_hash),
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                        "transaction error: failed to execute transaction"
                    );
                    return Err(e.wrap_err("transaction failed to execute"));
                }
            }

            // update current action group to tx's action group
            current_tx_group = tx_group;
        }

        Ok(execution_results)
    }

    /// sets up the state for execution of the block's transactions.
    /// set the current height and timestamp, and calls `begin_block` on all components.
    ///
    /// this *must* be called anytime before a block's txs are executed, whether it's
    /// during the proposal phase, or finalize_block phase.
    #[instrument(name = "App::pre_execute_transactions", skip_all, err)]
    async fn pre_execute_transactions(&mut self, block_data: BlockData) -> Result<()> {
        let chain_id = self
            .state_delta
            .get_chain_id()
            .await
            .wrap_err("failed to get chain ID from state")?;

        // reset recost flag
        self.recost_mempool = false;

        // call begin_block on all components
        // NOTE: the fields marked `unused` are not used by any of the components;
        // however, we need to still construct a `BeginBlock` type for now as
        // the penumbra IBC implementation still requires it as a parameter.
        let begin_block: abci::request::BeginBlock = abci::request::BeginBlock {
            hash: Hash::default(), // unused
            byzantine_validators: block_data.misbehavior.clone(),
            header: Header {
                app_hash: self.app_hash.clone(),
                chain_id: chain_id.clone(),
                consensus_hash: Hash::default(),      // unused
                data_hash: Some(Hash::default()),     // unused
                evidence_hash: Some(Hash::default()), // unused
                height: block_data.height,
                last_block_id: None,                      // unused
                last_commit_hash: Some(Hash::default()),  // unused
                last_results_hash: Some(Hash::default()), // unused
                next_validators_hash: block_data.next_validators_hash,
                proposer_address: block_data.proposer_address,
                time: block_data.time,
                validators_hash: Hash::default(), // unused
                version: tendermint::block::header::Version {
                    // unused
                    app: 0,
                    block: 0,
                },
            },
            last_commit_info: tendermint::abci::types::CommitInfo {
                round: 0u16.into(), // unused
                votes: vec![],
            }, // unused
        };

        self.begin_block(&begin_block)
            .await
            .wrap_err("begin_block failed")?;

        Ok(())
    }

    /// updates the app state after transaction execution, and generates the resulting
    /// `SequencerBlock`.
    ///
    /// this must be called after a block's transactions are executed.
    #[instrument(name = "App::post_execute_transactions", skip_all)]
    async fn post_execute_transactions(
        &mut self,
        block_hash: Hash,
        height: tendermint::block::Height,
        time: tendermint::Time,
        proposer_address: account::Id,
        txs: Vec<bytes::Bytes>,
        tx_results: Vec<ExecTxResult>,
    ) -> Result<()> {
        let Hash::Sha256(block_hash) = block_hash else {
            bail!("block hash is empty; this should not occur")
        };

        let chain_id = self
            .state_delta
            .get_chain_id()
            .await
            .wrap_err("failed to get chain ID from state")?;
        let sudo_address = self
            .state_delta
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;

        let end_block = self.end_block(height.value(), &sudo_address).await?;

        // get deposits for this block from state's ephemeral cache and put them to storage.
        let mut delta_delta = StateDelta::new(self.state_delta.clone());
        let deposits_in_this_block = self.state_delta.get_cached_block_deposits();
        debug!(
            deposits = %telemetry::display::json(&deposits_in_this_block),
            "got block deposits from state"
        );

        delta_delta
            .put_deposits(&block_hash, deposits_in_this_block.clone())
            .wrap_err("failed to put deposits to state")?;

        // cometbft expects a result for every tx in the block, so we need to return a
        // tx result for the commitments, even though they're not actually user txs.
        //
        // the tx_results passed to this function only contain results for every user
        // transaction, not the commitment, so its length is len(txs) - 2.
        let mut finalize_block_tx_results: Vec<ExecTxResult> = Vec::with_capacity(txs.len());
        finalize_block_tx_results.extend(std::iter::repeat(ExecTxResult::default()).take(2));
        finalize_block_tx_results.extend(tx_results);

        let sequencer_block = SequencerBlock::try_from_block_info_and_data(
            block_hash,
            chain_id,
            height,
            time,
            proposer_address,
            txs,
            deposits_in_this_block,
        )
        .wrap_err("failed to convert block info and data to SequencerBlock")?;
        delta_delta
            .put_sequencer_block(sequencer_block)
            .wrap_err("failed to write sequencer block to state")?;

        let result = PostTransactionExecutionResult {
            events: end_block.events,
            validator_updates: end_block.validator_updates,
            consensus_param_updates: end_block.consensus_param_updates,
            tx_results: finalize_block_tx_results,
        };

        delta_delta.object_put(POST_TRANSACTION_EXECUTION_RESULT_KEY, result);

        // events that occur after end_block are ignored here;
        // there should be none anyways.
        let _ = self.apply(delta_delta);

        Ok(())
    }

    /// Executes the given block, but does not write it to disk.
    ///
    /// `commit` must be called after this to write the block to disk.
    ///
    /// This is called by cometbft after the block has already been
    /// committed by the network's consensus.
    #[instrument(name = "App::finalize_block", skip_all)]
    pub(crate) async fn finalize_block(
        &mut self,
        finalize_block: abci::request::FinalizeBlock,
        storage: Storage,
    ) -> Result<abci::response::FinalizeBlock> {
        // If we previously executed txs in a different proposal than is being processed,
        // reset cached state changes.
        if self.executed_proposal_hash != finalize_block.hash {
            self.update_state_for_new_round(&storage);
        }

        ensure!(
            finalize_block.txs.len() >= 2,
            "block must contain at least two transactions: the rollup transactions commitment and
             rollup IDs commitment"
        );

        // When the hash is not empty, we have already executed and cached the results
        if self.executed_proposal_hash.is_empty() {
            // convert tendermint id to astria address; this assumes they are
            // the same address, as they are both ed25519 keys
            let proposer_address = finalize_block.proposer_address;
            let height = finalize_block.height;
            let time = finalize_block.time;

            // we haven't executed anything yet, so set up the state for execution.
            let block_data = BlockData {
                misbehavior: finalize_block.misbehavior,
                height,
                time,
                next_validators_hash: finalize_block.next_validators_hash,
                proposer_address,
            };

            self.pre_execute_transactions(block_data)
                .await
                .wrap_err("failed to execute block")?;

            let mut tx_results = Vec::with_capacity(finalize_block.txs.len());
            // skip the first two transactions, as they are the rollup data commitments
            for tx in finalize_block.txs.iter().skip(2) {
                let signed_tx = signed_transaction_from_bytes(tx)
                    .wrap_err("protocol error; only valid txs should be finalized")?;

                match self.execute_transaction(Arc::new(signed_tx)).await {
                    Ok(events) => tx_results.push(ExecTxResult {
                        events,
                        ..Default::default()
                    }),
                    Err(e) => {
                        // this is actually a protocol error, as only valid txs should be finalized
                        tracing::error!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            "failed to finalize transaction; ignoring it",
                        );
                        let code = if e.downcast_ref::<InvalidNonce>().is_some() {
                            AbciErrorCode::INVALID_NONCE
                        } else {
                            AbciErrorCode::INTERNAL_ERROR
                        };
                        tx_results.push(ExecTxResult {
                            code: Code::Err(code.value()),
                            info: code.info(),
                            log: format!("{e:#}"),
                            ..Default::default()
                        });
                    }
                }
            }

            self.post_execute_transactions(
                finalize_block.hash,
                height,
                time,
                proposer_address,
                finalize_block.txs,
                tx_results,
            )
            .await
            .wrap_err("failed to run post execute transactions handler")?;
        }

        // update the priority of any txs in the mempool based on the updated app state
        if self.recost_mempool {
            self.metrics.increment_mempool_recosted();
        }
        update_mempool_after_finalization(
            &mut self.mempool,
            &self.state_delta,
            self.recost_mempool,
        )
        .await;

        let post_transaction_execution_result: PostTransactionExecutionResult = self
            .state_delta
            .object_get(POST_TRANSACTION_EXECUTION_RESULT_KEY)
            .expect(
                "post_transaction_execution_result must be present, as txs were already executed \
                 just now or during the proposal phase",
            );

        // prepare the `StagedWriteBatch` for a later commit.
        let app_hash = self
            .prepare_commit(storage)
            .await
            .wrap_err("failed to prepare commit")?;
        let finalize_block = abci::response::FinalizeBlock {
            events: post_transaction_execution_result.events,
            validator_updates: post_transaction_execution_result.validator_updates,
            consensus_param_updates: post_transaction_execution_result.consensus_param_updates,
            app_hash,
            tx_results: post_transaction_execution_result.tx_results,
        };

        Ok(finalize_block)
    }

    #[instrument(skip_all, err)]
    async fn prepare_commit(&mut self, storage: Storage) -> Result<AppHash> {
        // extract the state we've built up to so we can prepare it as a `StagedWriteBatch`.
        let dummy_state = storage.new_delta_of_latest_snapshot();
        let mut state_delta = Arc::try_unwrap(std::mem::replace(
            &mut self.state_delta,
            Arc::new(dummy_state),
        ))
        .expect("we have exclusive ownership of the State at commit()");

        // store the storage version indexed by block height
        let new_version = storage.latest_version().wrapping_add(1);
        let height = state_delta
            .get_block_height()
            .await
            .expect("block height must be set, as `put_block_height` was already called");
        state_delta
            .put_storage_version_by_height(height, new_version)
            .wrap_err("failed to put storage version by height")?;
        debug!(
            height,
            version = new_version,
            "stored storage version for height"
        );

        let write_batch = storage
            .prepare_commit(state_delta)
            .await
            .wrap_err("failed to prepare commit")?;
        let app_hash: AppHash = write_batch
            .root_hash()
            .0
            .to_vec()
            .try_into()
            .wrap_err("failed to convert app hash")?;
        self.write_batch = Some(write_batch);
        Ok(app_hash)
    }

    #[instrument(name = "App::begin_block", skip_all)]
    async fn begin_block(
        &mut self,
        begin_block: &abci::request::BeginBlock,
    ) -> Result<Vec<abci::Event>> {
        let mut delta_delta = StateDelta::new(self.state_delta.clone());

        delta_delta
            .put_block_height(begin_block.header.height.into())
            .wrap_err("failed to put block height")?;
        delta_delta
            .put_block_timestamp(begin_block.header.time)
            .wrap_err("failed to put block timestamp")?;

        // call begin_block on all components
        let mut arc_delta_delta = Arc::new(delta_delta);
        AccountsComponent::begin_block(&mut arc_delta_delta, begin_block)
            .await
            .wrap_err("begin_block failed on AccountsComponent")?;
        AuthorityComponent::begin_block(&mut arc_delta_delta, begin_block)
            .await
            .wrap_err("begin_block failed on AuthorityComponent")?;
        IbcComponent::begin_block(&mut arc_delta_delta, begin_block)
            .await
            .wrap_err("begin_block failed on IbcComponent")?;
        FeesComponent::begin_block(&mut arc_delta_delta, begin_block)
            .await
            .wrap_err("begin_block failed on FeesComponent")?;

        let delta_delta = Arc::try_unwrap(arc_delta_delta)
            .expect("components should not retain copies of shared state");

        Ok(self.apply(delta_delta))
    }

    /// Executes a signed transaction.
    #[instrument(name = "App::execute_transaction", skip_all)]
    async fn execute_transaction(&mut self, signed_tx: Arc<Transaction>) -> Result<Vec<Event>> {
        signed_tx
            .check_stateless()
            .await
            .wrap_err("stateless check failed")?;

        let mut delta_delta = self
            .state_delta
            .try_begin_transaction()
            .expect("state Arc should be present and unique");

        signed_tx
            .check_and_execute(&mut delta_delta)
            .await
            .wrap_err("failed executing transaction")?;

        // flag mempool for cleaning if we ran a fee change action
        self.recost_mempool = self.recost_mempool
            || signed_tx.is_bundleable_sudo_action_group()
                && signed_tx
                    .actions()
                    .iter()
                    .any(|act| act.is_fee_asset_change() || act.is_fee_change());

        Ok(delta_delta.apply().1)
    }

    #[instrument(name = "App::end_block", skip_all)]
    async fn end_block(
        &mut self,
        height: u64,
        fee_recipient: &[u8; 20],
    ) -> Result<abci::response::EndBlock> {
        let delta_delta = StateDelta::new(self.state_delta.clone());
        let mut arc_delta_delta = Arc::new(delta_delta);

        let end_block = abci::request::EndBlock {
            height: height
                .try_into()
                .expect("a block height should be able to fit in an i64"),
        };

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_delta_delta, &end_block)
            .await
            .wrap_err("end_block failed on AccountsComponent")?;
        AuthorityComponent::end_block(&mut arc_delta_delta, &end_block)
            .await
            .wrap_err("end_block failed on AuthorityComponent")?;
        FeesComponent::end_block(&mut arc_delta_delta, &end_block)
            .await
            .wrap_err("end_block failed on FeesComponent")?;
        IbcComponent::end_block(&mut arc_delta_delta, &end_block)
            .await
            .wrap_err("end_block failed on IbcComponent")?;

        let mut delta_delta = Arc::try_unwrap(arc_delta_delta)
            .expect("components should not retain copies of shared state");

        // gather and return validator updates
        let validator_updates = self
            .state_delta
            .get_validator_updates()
            .await
            .expect("failed getting validator updates");

        // clear validator updates
        delta_delta.clear_validator_updates();

        // gather block fees and transfer them to the block proposer
        let fees = self.state_delta.get_block_fees();

        for fee in fees {
            delta_delta
                .increase_balance(fee_recipient, fee.asset(), fee.amount())
                .await
                .wrap_err("failed to increase fee recipient balance")?;
        }

        let events = self.apply(delta_delta);
        Ok(abci::response::EndBlock {
            validator_updates: validator_updates
                .try_into_cometbft()
                .wrap_err("failed converting astria validators to cometbft compatible type")?,
            events,
            ..Default::default()
        })
    }

    #[instrument(name = "App::commit", skip_all)]
    pub(crate) async fn commit(&mut self, storage: Storage) {
        // Commit the pending writes, clearing the state.
        let app_hash = storage
            .commit_batch(self.write_batch.take().expect(
                "write batch must be set, as `finalize_block` is always called before `commit`",
            ))
            .expect("must be able to successfully commit to storage");
        tracing::debug!(
            app_hash = %telemetry::display::hex(&app_hash),
            "finished committing state",
        );
        self.app_hash = app_hash
            .0
            .to_vec()
            .try_into()
            .expect("root hash to app hash conversion must succeed");

        // Get the latest version of the state, now that we've committed it.
        self.state_delta = Arc::new(storage.new_delta_of_latest_snapshot());
    }

    // StateDelta::apply only works when the StateDelta wraps an underlying
    // StateWrite.  But if we want to share the StateDelta with spawned tasks,
    // we usually can't wrap a StateWrite instance, which requires exclusive
    // access. This method "externally" applies the state delta to the
    // inter-block state.
    //
    // Invariant: delta_delta and self.state are the only two references to the
    // inter-block state.
    fn apply(&mut self, delta_delta: StateDelta<InterBlockState>) -> Vec<Event> {
        let (state2, mut cache) = delta_delta.flatten();
        drop(state2);
        // Now there is only one reference to the inter-block state: self.state

        let events = cache.take_events();
        cache.apply_to(
            Arc::get_mut(&mut self.state_delta).expect("no other references to inter-block state"),
        );

        events
    }
}

// updates the mempool to reflect current state
//
// NOTE: this function locks the mempool until all accounts have been cleaned.
// this could potentially stall consensus from moving to the next round if
// the mempool is large, especially if recosting transactions.
async fn update_mempool_after_finalization<S: StateRead>(
    mempool: &mut Mempool,
    state: &S,
    recost: bool,
) {
    mempool.run_maintenance(state, recost).await;
}

/// relevant data of a block being executed.
///
/// used to setup the state before execution of transactions.
#[derive(Debug, Clone)]
struct BlockData {
    misbehavior: Vec<tendermint::abci::types::Misbehavior>,
    height: tendermint::block::Height,
    time: tendermint::Time,
    next_validators_hash: Hash,
    proposer_address: account::Id,
}

fn signed_transaction_from_bytes(bytes: &[u8]) -> Result<Transaction> {
    let raw = raw::Transaction::decode(bytes)
        .wrap_err("failed to decode protobuf to signed transaction")?;
    let tx = Transaction::try_from_raw(raw)
        .wrap_err("failed to transform raw signed transaction to verified type")?;

    Ok(tx)
}

#[derive(Clone, Debug)]
struct PostTransactionExecutionResult {
    events: Vec<Event>,
    tx_results: Vec<ExecTxResult>,
    validator_updates: Vec<tendermint::validator::Update>,
    consensus_param_updates: Option<tendermint::consensus::Params>,
}
