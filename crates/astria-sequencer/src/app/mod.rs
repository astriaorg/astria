#[cfg(feature = "benchmark")]
mod benchmarks;
pub(crate) mod event_bus;
mod execution_state;
mod state_ext;
pub(crate) mod storage;
#[cfg(test)]
mod tests_app;
#[cfg(test)]
mod tests_block_ordering;
#[cfg(test)]
mod tests_breaking_changes;

pub(crate) mod vote_extension;

use std::{
    collections::HashSet,
    sync::Arc,
    time::Instant,
};

use astria_core::{
    primitive::v1::{
        RollupId,
        TransactionId,
    },
    protocol::{
        abci::AbciErrorCode,
        genesis::v1::GenesisAppState,
        price_feed::v1::ExtendedCommitInfoWithCurrencyPairMapping,
        transaction::v1::action::{
            group::Group,
            ValidatorUpdate,
        },
    },
    sequencerblock::v1::{
        block::{
            self,
            ExpandedBlockData,
            SequencerBlockBuilder,
        },
        DataItem,
        SequencerBlock,
    },
    upgrades::v1::ChangeHash,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        ensure,
        Report,
        Result,
        WrapErr as _,
    },
};
use bytes::Bytes;
use cnidarium::{
    ArcStateDeltaExt,
    Snapshot,
    StagedWriteBatch,
    StateDelta,
    StateRead,
    StateWrite,
    Storage,
};
use futures::future::try_join_all;
use prost::Message as _;
use telemetry::display::{
    base64,
    json,
};
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
    Time,
};
use tracing::{
    debug,
    info,
    instrument,
    trace,
    warn,
    Level,
};

pub(crate) use self::state_ext::{
    StateReadExt,
    StateWriteExt,
};
use crate::{
    accounts::{
        component::AccountsComponent,
        StateWriteExt as _,
    },
    address::StateWriteExt as _,
    app::{
        event_bus::{
            EventBus,
            EventBusSubscription,
        },
        execution_state::{
            ExecutionState,
            ExecutionStateMachine,
        },
        vote_extension::ProposalHandler,
    },
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
    checked_actions::CheckedAction,
    checked_transaction::{
        CheckedTransaction,
        CheckedTransactionExecutionError,
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
        commitment::generate_rollup_datas_commitment,
    },
    upgrades::UpgradesHandler,
};

// ephemeral store key for the cache of results of executing of transactions in `prepare_proposal`.
// cleared in `process_proposal` if we're the proposer.
const EXECUTION_RESULTS_KEY: &str = "execution_results";

// ephemeral store key for the cache of results of executing of transactions in `process_proposal`.
// cleared at the end of the block.
const POST_TRANSACTION_EXECUTION_RESULT_KEY: &str = "post_transaction_execution_result";

// the height to set the `vote_extensions_enable_height` to in state if vote extensions are
// disabled.
const VOTE_EXTENSIONS_DISABLED_HEIGHT: u64 = 0;

/// The inter-block state being written to by the application.
type InterBlockState = Arc<StateDelta<Snapshot>>;

/// The Sequencer application, written as a bundle of [`Component`]s.
///
/// Note: this is called `App` because this is a Tendermint ABCI application,
/// and implements the state transition logic of the chain.
///
/// See also the [Penumbra reference] implementation.
///
/// [Penumbra reference]: https://github.com/penumbra-zone/penumbra/blob/9cc2c644e05c61d21fdc7b507b96016ba6b9a935/app/src/app/mod.rs#L42
pub(crate) struct App {
    state: InterBlockState,

    // The mempool of the application.
    //
    // Transactions are pulled from this mempool during `prepare_proposal`.
    mempool: Mempool,

    // TODO(https://github.com/astriaorg/astria/issues/1660): use the ephemeral
    // storage to track this instead of a field on the app object.

    // An identifier for the given app's status through execution of different ABCI
    // calls.
    //
    // Used to avoid double execution of transactions across ABCI calls, as well as
    // to indicate when we must clear and re-execute (ie if round has changed).
    execution_state: ExecutionStateMachine,

    // This is set when a `FeeChange` or `FeeAssetChange` action is seen in a block to flag
    // to the mempool to recost all transactions.
    recost_mempool: bool,

    // this is committed to the state when `commit` is called, and set to `None`.
    write_batch: Option<WriteBatch>,

    // the currently committed `AppHash` of the application state.
    // set whenever `commit` is called.
    #[expect(
        clippy::struct_field_names,
        reason = "we need to be specific as to what hash this is"
    )]
    app_hash: AppHash,

    // the sequencer event bus, used to send and receive events between components within the app
    event_bus: EventBus,

    upgrades_handler: UpgradesHandler,

    // used to create and verify vote extensions, if this is a validator node.
    vote_extension_handler: vote_extension::Handler,

    metrics: &'static Metrics,
}

/// A wrapper around [`StagedWriteBatch`] which includes other information necessary for commitment.
struct WriteBatch {
    /// The current `StagedWriteBatch` which contains the rocksdb write batch
    /// of the current block being executed, created from the state delta,
    /// and set after `finalize_block`.
    write_batch: StagedWriteBatch,
    /// The IDs of all transactions executed in the block for which this write batch is made.
    executed_tx_ids: HashSet<TransactionId>,
}

impl App {
    #[instrument(name = "App::new", skip_all, err)]
    pub(crate) async fn new(
        snapshot: Snapshot,
        mempool: Mempool,
        upgrades_handler: UpgradesHandler,
        vote_extension_handler: vote_extension::Handler,
        metrics: &'static Metrics,
    ) -> Result<Self> {
        debug!("initializing App instance");

        let app_hash: AppHash = snapshot
            .root_hash()
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get current root hash")?
            .0
            .to_vec()
            .try_into()
            .expect("root hash conversion must succeed; should be 32 bytes");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        let event_bus = EventBus::new();

        Ok(Self {
            state,
            mempool,
            execution_state: ExecutionStateMachine::new(),
            recost_mempool: false,
            write_batch: None,
            app_hash,
            event_bus,
            upgrades_handler,
            vote_extension_handler,
            metrics,
        })
    }

    pub(crate) fn subscribe_to_events(&self) -> EventBusSubscription {
        self.event_bus.subscribe()
    }

    #[instrument(name = "App:init_chain", skip_all, err)]
    pub(crate) async fn init_chain(
        &mut self,
        storage: Storage,
        genesis_state: GenesisAppState,
        genesis_validators: Vec<ValidatorUpdate>,
        chain_id: String,
    ) -> Result<AppHash> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        state_tx
            .put_base_prefix(genesis_state.address_prefixes().base().to_string())
            .wrap_err("failed to write base prefix to state")?;
        state_tx
            .put_ibc_compat_prefix(genesis_state.address_prefixes().ibc_compat().to_string())
            .wrap_err("failed to write ibc-compat prefix to state")?;

        if let Some(native_asset) = genesis_state.native_asset_base_denomination() {
            state_tx
                .put_native_asset(native_asset.clone())
                .wrap_err("failed to write native asset to state")?;
            state_tx
                .put_ibc_asset(native_asset.clone())
                .wrap_err("failed to commit native asset as ibc asset to state")?;
        }

        state_tx
            .put_chain_id_and_revision_number(chain_id.try_into().context("invalid chain ID")?)
            .wrap_err("failed to write chain id to state")?;
        state_tx
            .put_block_height(0)
            .wrap_err("failed to write block height to state")?;

        // call init_chain on all components
        FeesComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .wrap_err("init_chain failed on FeesComponent")?;
        AccountsComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .wrap_err("init_chain failed on AccountsComponent")?;
        AuthorityComponent::init_chain(
            &mut state_tx,
            &AuthorityComponentAppState {
                authority_sudo_address: *genesis_state.authority_sudo_address(),
                genesis_validators,
            },
        )
        .await
        .wrap_err("init_chain failed on AuthorityComponent")?;
        IbcComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .wrap_err("init_chain failed on IbcComponent")?;

        state_tx.apply();

        let app_hash = self
            .prepare_commit(storage, HashSet::new())
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
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        self.execution_state = ExecutionStateMachine::new();
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions.
    ///
    /// This is required so that a rollup can easily verify that the transactions it
    /// receives are correct (ie. we actually included in a sequencer block, and none
    /// are missing)
    /// It puts this special "commitment" as the first transaction in a block.
    /// When other validators receive the block, they know the first transaction is
    /// supposed to be the commitment, and verifies that is it correct.
    #[instrument(name = "App::prepare_proposal", skip_all, err(level = Level::WARN))]
    pub(crate) async fn prepare_proposal(
        &mut self,
        prepare_proposal: abci::request::PrepareProposal,
        storage: Storage,
    ) -> Result<abci::response::PrepareProposal> {
        // Always reset when preparing a proposal.
        self.update_state_for_new_round(&storage);

        let request = prepare_proposal.clone();
        let block_data = BlockData {
            misbehavior: prepare_proposal.misbehavior,
            height: prepare_proposal.height,
            time: prepare_proposal.time,
            next_validators_hash: prepare_proposal.next_validators_hash,
            proposer_address: prepare_proposal.proposer_address,
        };

        let upgrade_change_hashes = self
            .pre_execute_transactions(block_data)
            .await
            .wrap_err("failed to prepare for executing block")?;
        let encoded_upgrade_change_hashes = if upgrade_change_hashes.is_empty() {
            None
        } else {
            Some(DataItem::UpgradeChangeHashes(upgrade_change_hashes).encode())
        };

        let uses_data_item_enum = self.uses_data_item_enum(prepare_proposal.height);
        let mut block_size_constraints =
            BlockSizeConstraints::new(prepare_proposal.max_tx_bytes, uses_data_item_enum)
                .wrap_err("failed to create block size constraints")?;
        if let Some(bytes) = &encoded_upgrade_change_hashes {
            block_size_constraints
                .cometbft_checked_add(bytes.len())
                .wrap_err("exceeded size limit while adding upgrade change hashes")?;
        }

        let vote_extensions_enabled = self
            .vote_extensions_enabled(prepare_proposal.height)
            .await?;
        let encoded_extended_commit_info = if vote_extensions_enabled {
            // create the extended commit info from the local last commit
            let Some(last_commit) = prepare_proposal.local_last_commit else {
                bail!("local last commit is empty; this should not occur")
            };

            // if this fails, we shouldn't return an error, but instead leave
            // the vote extensions empty in this block for liveness.
            // it's not a critical error if the oracle values are not updated for a block.
            //
            // note that at the height where vote extensions are enabled, the `extended_commit_info`
            // will always be empty, as there were no vote extensions for the previous block.
            let round = last_commit.round;
            let extended_commit_info = ProposalHandler::prepare_proposal(
                &self.state,
                prepare_proposal.height.into(),
                last_commit,
            )
            .await
            .unwrap_or_else(|error| {
                warn!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&error),
                    "failed to generate extended commit info"
                );
                ExtendedCommitInfoWithCurrencyPairMapping::empty(round)
            });

            let mut encoded_extended_commit_info = DataItem::ExtendedCommitInfo(
                extended_commit_info.into_raw().encode_to_vec().into(),
            )
            .encode();

            if block_size_constraints
                .cometbft_checked_add(encoded_extended_commit_info.len())
                .is_err()
            {
                // We would exceed the CometBFT size limit - try just adding an empty extended
                // commit info rather than erroring out to ensure liveness.
                warn!(
                    encoded_extended_commit_info_len = encoded_extended_commit_info.len(),
                    "extended commit info is too large to fit in block; not including in block"
                );
                encoded_extended_commit_info = DataItem::ExtendedCommitInfo(Bytes::new()).encode();
                block_size_constraints
                    .cometbft_checked_add(encoded_extended_commit_info.len())
                    .wrap_err("exceeded size limit while adding empty extended commit info")?;
            }

            Some(encoded_extended_commit_info)
        } else {
            None
        };

        // ignore the txs passed by cometbft in favour of our app-side mempool
        let included_txs = self
            .prepare_proposal_tx_execution(block_size_constraints)
            .await
            .wrap_err("failed to execute transactions")?;
        self.metrics
            .record_proposal_transactions(included_txs.len());

        let deposits = self.state.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        // generate commitment to sequence::Actions and deposits and commitment to the rollup IDs
        // included in the block, chain on the extended commit info if `Some`, and finally chain on
        // the tx bytes.
        let commitments_iter = if uses_data_item_enum {
            generate_rollup_datas_commitment::<true>(&included_txs, deposits).into_iter()
        } else {
            generate_rollup_datas_commitment::<false>(&included_txs, deposits).into_iter()
        };

        let included_txs_encoded_bytes = included_txs
            .iter()
            .map(|checked_tx| checked_tx.encoded_bytes().clone());
        let txs = commitments_iter
            .chain(encoded_upgrade_change_hashes.into_iter())
            .chain(encoded_extended_commit_info.into_iter())
            .chain(included_txs_encoded_bytes)
            .collect();

        let response = abci::response::PrepareProposal {
            txs,
        };

        // Generate the prepared proposal fingerprint.
        self.execution_state
            .set_prepared_proposal(request, response.clone())
            .wrap_err("failed to set executed proposal fingerprint, this should not happen")?;
        Ok(response)
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions
    /// and ensures it matches the commitment created by the proposer, which
    /// should be the first transaction in the block.
    #[instrument(
        name = "App::process_proposal",
        skip_all,
        fields(proposer=%base64(&process_proposal.proposer_address.as_bytes())),
        err(level = Level::WARN)
    )]
    pub(crate) async fn process_proposal(
        &mut self,
        process_proposal: abci::request::ProcessProposal,
        storage: Storage,
    ) -> Result<()> {
        // Check the proposal against the prepared proposal fingerprint.
        let skip_execution = self
            .execution_state
            .check_if_prepared_proposal(process_proposal.clone());

        // Based on the status after the check, a couple of logs and metrics may
        // be updated or emitted.
        match self.execution_state.data() {
            // The proposal was prepared by this node, so we skip execution.
            ExecutionState::PreparedValid(_) => {
                trace!("skipping process_proposal as we are the proposer for this block");
            }
            ExecutionState::Prepared(_) => {
                bail!("prepared proposal fingerprint was not validated, this should not happen")
            }
            // We have a cached proposal from prepare proposal, but it does not match
            // the current proposal. We should clear the cache and execute proposal.
            //
            // This can happen in HA nodes, but if happening in single nodes likely a bug.
            ExecutionState::CheckedPreparedMismatch(_) => {
                self.metrics.increment_process_proposal_skipped_proposal();
                trace!(
                    "there was a previously prepared proposal cached, but did not match current \
                     proposal, will clear and execute block"
                );
            }
            // There was a previously executed full block cached, likely indicates
            // a new round of voting has started. Previous proposal may have failed.
            ExecutionState::ExecutedBlock {
                cached_block_hash: _,
                cached_proposal,
            }
            | ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash: _,
                cached_proposal,
            } => {
                if cached_proposal.is_none() {
                    trace!(
                        "there was a previously executed block cached, but no proposal hash, will \
                         clear and execute"
                    );
                } else {
                    trace!(
                        "our prepared proposal cache executed fully, but was not committed, will \
                         clear and execute"
                    );
                }
            }
            // No cached proposal, nothing to do for logging. Common case for validator voting.
            ExecutionState::Unset => {}
        }

        let uses_data_item_enum = self.uses_data_item_enum(process_proposal.height);
        let expanded_block_data = if uses_data_item_enum {
            let with_extended_commit_info = self
                .vote_extensions_enabled(process_proposal.height)
                .await?;
            ExpandedBlockData::new_from_typed_data(&process_proposal.txs, with_extended_commit_info)
        } else {
            ExpandedBlockData::new_from_untyped_data(&process_proposal.txs)
        }
        .wrap_err("failed to parse data items")?;

        // If we can skip execution just fetch the cache, otherwise need to run the execution.
        let (rollup_data_bytes, tx_results, tx_ids) = if skip_execution {
            // if we're the proposer, we should have the execution results from
            // `prepare_proposal`. run the post-tx-execution hook to generate the
            // `SequencerBlock` and to set `self.finalize_block`.
            //
            // we can't run this in `prepare_proposal` as we don't know the block hash there.
            let Some((rollup_data_bytes, tx_results, tx_ids)) =
                self.state.object_get(EXECUTION_RESULTS_KEY)
            else {
                bail!("execution results must be present after executing transactions")
            };

            (rollup_data_bytes, tx_results, tx_ids)
        } else {
            self.update_state_for_new_round(&storage);

            if let Some(extended_commit_info_with_proof) =
                &expanded_block_data.extended_commit_info_with_proof
            {
                let Some(last_commit) = process_proposal.proposed_last_commit else {
                    bail!("proposed last commit is empty; this should not occur")
                };

                // validate the extended commit info
                ProposalHandler::validate_proposal(
                    &self.state,
                    process_proposal.height.value(),
                    &last_commit,
                    extended_commit_info_with_proof.extended_commit_info(),
                )
                .await
                .wrap_err("failed to validate extended commit info")?;
            }

            let block_data = BlockData {
                misbehavior: process_proposal.misbehavior,
                height: process_proposal.height,
                time: process_proposal.time,
                next_validators_hash: process_proposal.next_validators_hash,
                proposer_address: process_proposal.proposer_address,
            };

            let upgrade_change_hashes = self
                .pre_execute_transactions(block_data)
                .await
                .wrap_err("failed to prepare for executing block")?;
            ensure_upgrade_change_hashes_as_expected(
                &expanded_block_data,
                upgrade_change_hashes.as_ref(),
            )?;

            // we don't care about the cometbft max_tx_bytes here, as cometbft would have
            // rejected the proposal if it was too large.
            // however, we should still validate the other constraints, namely
            // the max sequenced data bytes.
            let block_size_constraints = BlockSizeConstraints::new_unlimited_cometbft();

            let user_submitted_transactions = construct_checked_txs(
                &expanded_block_data.user_submitted_transactions,
                &self.state,
            )
            .await
            .wrap_err("failed to construct checked transactions in process proposal")?;
            let (tx_results, tx_ids) = self
                .process_proposal_tx_execution(&user_submitted_transactions, block_size_constraints)
                .await
                .wrap_err("failed to execute transactions in process proposal")?;

            self.metrics
                .record_proposal_transactions(user_submitted_transactions.len());

            let deposits = self.state.get_cached_block_deposits();
            self.metrics.record_proposal_deposits(deposits.len());

            let (expected_rollup_datas_root, expected_rollup_ids_root) = if uses_data_item_enum {
                let commitments = generate_rollup_datas_commitment::<true>(
                    &user_submitted_transactions,
                    deposits,
                );
                (commitments.rollup_datas_root, commitments.rollup_ids_root)
            } else {
                let commitments = generate_rollup_datas_commitment::<false>(
                    &user_submitted_transactions,
                    deposits,
                );
                (commitments.rollup_datas_root, commitments.rollup_ids_root)
            };
            ensure!(
                expanded_block_data.rollup_transactions_root == expected_rollup_datas_root,
                "rollup transactions commitment does not match expected",
            );
            ensure!(
                expanded_block_data.rollup_ids_root == expected_rollup_ids_root,
                "rollup IDs commitment does not match expected",
            );

            let rollup_data_bytes = user_submitted_transactions
                .iter()
                .flat_map(|checked_tx| {
                    checked_tx
                        .rollup_data_bytes()
                        .map(|(rollup_id, data)| (*rollup_id, data.clone()))
                })
                .collect();
            (rollup_data_bytes, tx_results, tx_ids)
        };

        let sequencer_block = self
            .post_execute_transactions(
                process_proposal.hash,
                process_proposal.height,
                process_proposal.time,
                process_proposal.proposer_address,
                expanded_block_data,
                rollup_data_bytes,
                tx_results,
                tx_ids,
            )
            .await
            .wrap_err("failed to run post execute transactions handler")?;

        self.event_bus
            .send_process_proposal_block(Arc::new(sequencer_block));

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
    #[instrument(name = "App::prepare_proposal_tx_execution", skip_all, err(level = Level::DEBUG))]
    async fn prepare_proposal_tx_execution(
        &mut self,
        block_size_constraints: BlockSizeConstraints,
    ) -> Result<Vec<Arc<CheckedTransaction>>> {
        let mempool_len = self.mempool.len().await;
        debug!(mempool_len, "executing transactions from mempool");

        let mut proposal_info = Proposal::Prepare {
            block_size_constraints,
            included_txs: Vec::new(),
            failed_tx_count: 0,
            execution_results: Vec::new(),
            executed_tx_ids: HashSet::new(),
            excluded_tx_count: 0,
            current_tx_group: Group::BundleableGeneral,
            mempool: self.mempool.clone(),
            metrics: self.metrics,
        };

        // get copy of transactions to execute from mempool
        let pending_txs = self.mempool.builder_queue().await;

        let mut unused_count = pending_txs.len();
        let mut rollup_data_bytes = vec![];
        for tx in pending_txs {
            unused_count = unused_count.saturating_sub(1);
            rollup_data_bytes.extend(
                tx.rollup_data_bytes()
                    .map(|(rollup_id, data)| (*rollup_id, data.clone())),
            );

            if self
                .proposal_checks_and_tx_execution(tx, &mut proposal_info)
                .await?
                .should_break()
            {
                break;
            }
        }

        let Proposal::Prepare {
            included_txs,
            failed_tx_count,
            execution_results,
            executed_tx_ids,
            excluded_tx_count,
            ..
        } = proposal_info
        else {
            bail!("expected `Proposal::Prepare`, received `Proposal::Process`")
        };

        if failed_tx_count > 0 {
            info!(
                failed_tx_count = failed_tx_count,
                included_tx_count = included_txs.len(),
                "excluded transactions from block due to execution failure"
            );
        }
        self.metrics.set_prepare_proposal_excluded_transactions(
            excluded_tx_count.saturating_add(failed_tx_count),
        );

        debug!("{unused_count} leftover pending transactions");
        self.metrics
            .set_transactions_in_mempool_total(self.mempool.len().await);

        // XXX: we need to unwrap the app's state arc to write
        // to the ephemeral store.
        // this is okay as we should have the only reference to the state
        // at this point.
        let mut state_tx = Arc::try_begin_transaction(&mut self.state)
            .expect("state Arc should not be referenced elsewhere");
        state_tx.object_put(
            EXECUTION_RESULTS_KEY,
            (rollup_data_bytes, execution_results, executed_tx_ids),
        );
        let _ = state_tx.apply();

        Ok(included_txs)
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
    #[instrument(name = "App::process_proposal_tx_execution", skip_all, err(level = Level::DEBUG))]
    async fn process_proposal_tx_execution(
        &mut self,
        txs: &[Arc<CheckedTransaction>],
        block_size_constraints: BlockSizeConstraints,
    ) -> Result<(Vec<ExecTxResult>, HashSet<TransactionId>)> {
        let mut proposal_info = Proposal::Process {
            block_size_constraints,
            execution_results: vec![],
            current_tx_group: Group::BundleableGeneral,
            executed_tx_ids: HashSet::new(),
        };

        for tx in txs {
            if self
                .proposal_checks_and_tx_execution(tx.clone(), &mut proposal_info)
                .await?
                .should_break()
            {
                break;
            }
        }
        Ok(proposal_info.execution_results_and_tx_ids())
    }

    #[instrument(skip_all)]
    async fn proposal_checks_and_tx_execution(
        &mut self,
        tx: Arc<CheckedTransaction>,
        proposal_info: &mut Proposal,
    ) -> Result<BreakOrContinue> {
        let tx_len = tx.encoded_bytes().len();
        info!(tx_id = %tx.id(), "executing transaction");

        // check CometBFT size constraints for `prepare_proposal`
        if let Proposal::Prepare {
            block_size_constraints,
            metrics,
            excluded_tx_count,
            ..
        } = proposal_info
        {
            if !block_size_constraints.cometbft_has_space(tx_len) {
                metrics.increment_prepare_proposal_excluded_transactions_cometbft_space();
                debug!(
                    tx_id = %tx.id(),
                    block_size_constraints = %json(block_size_constraints),
                    tx_data_bytes = tx_len,
                    "excluding remaining transactions: max cometBFT data limit reached"
                );
                *excluded_tx_count = excluded_tx_count.saturating_add(1);

                // break from calling loop, as the block is full
                return Ok(BreakOrContinue::Break);
            }
        }

        let debug_msg = match proposal_info {
            Proposal::Prepare {
                ..
            } => "excluding transaction",
            Proposal::Process {
                ..
            } => "transaction error",
        };

        // check sequencer size constraints
        let tx_sequence_data_length = tx
            .rollup_data_bytes()
            .map(|(_rollup_id, data)| data.len())
            .sum();
        if !proposal_info
            .block_size_constraints()
            .sequencer_has_space(tx_sequence_data_length)
        {
            debug!(
                tx_id = %tx.id(),
                block_size_constraints = %json(&proposal_info.block_size_constraints()),
                tx_data_length = tx_sequence_data_length,
                "{debug_msg}: max block sequenced data limit reached"
            );
            match proposal_info {
                Proposal::Prepare {
                    metrics,
                    excluded_tx_count,
                    ..
                } => {
                    metrics.increment_prepare_proposal_excluded_transactions_sequencer_space();
                    *excluded_tx_count = excluded_tx_count.saturating_add(1);

                    // continue as there might be non-sequence txs that can fit
                    return Ok(BreakOrContinue::Continue);
                }
                Proposal::Process {
                    ..
                } => bail!("max block sequenced data limit passed"),
            };
        }

        // ensure transaction's group is less than or equal to current action group
        let tx_group = tx.group();
        if tx_group > proposal_info.current_tx_group() {
            debug!(
                tx_id = %tx.id(),
                "{debug_msg}: group is higher priority than previously included transactions"
            );
            match proposal_info {
                Proposal::Prepare {
                    excluded_tx_count, ..
                } => {
                    *excluded_tx_count = excluded_tx_count.saturating_add(1);
                    return Ok(BreakOrContinue::Continue);
                }
                Proposal::Process {
                    ..
                } => {
                    bail!("transactions have incorrect transaction group ordering");
                }
            };
        }

        let (execution_results, executed_tx_ids) = proposal_info.execution_results_and_tx_ids_mut();
        match self.execute_transaction(tx.clone()).await {
            Ok(events) => {
                execution_results.push(ExecTxResult {
                    events,
                    ..Default::default()
                });
                executed_tx_ids.insert(*tx.id());
                proposal_info
                    .block_size_constraints_mut()
                    .sequencer_checked_add(tx_sequence_data_length)
                    .wrap_err("error growing sequencer block size")?;
                proposal_info
                    .block_size_constraints_mut()
                    .cometbft_checked_add(tx_len)
                    .wrap_err("error growing cometBFT block size")?;
                if let Proposal::Prepare {
                    included_txs, ..
                } = proposal_info
                {
                    included_txs.push(tx.clone());
                }
            }
            Err(error) => {
                debug!(
                    tx_id = %tx.id(),
                    %error,
                    "{debug_msg}: failed to execute transaction"
                );
                match proposal_info {
                    Proposal::Prepare {
                        metrics,
                        failed_tx_count,
                        mempool,
                        ..
                    } => {
                        metrics.increment_prepare_proposal_excluded_transactions_failed_execution();
                        if matches!(error, CheckedTransactionExecutionError::InvalidNonce { .. }) {
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
                            *failed_tx_count = failed_tx_count.saturating_add(1);

                            // remove the failing transaction from the mempool
                            //
                            // this will remove any transactions from the same sender
                            // as well, as the dependent nonces will not be able
                            // to execute
                            mempool
                                .remove_tx_invalid(
                                    tx,
                                    RemovalReason::FailedPrepareProposal(error.to_string()),
                                )
                                .await;
                        }
                    }
                    Proposal::Process {
                        ..
                    } => return Err(error).wrap_err("transaction failed to execute"),
                }
            }
        };
        proposal_info.set_current_tx_group(tx_group);
        Ok(BreakOrContinue::Continue)
    }

    /// Sets up the state for execution of the block's transactions.
    ///
    /// Executes any upgrade with an activation height of this block height, sets the current
    /// height and timestamp, and calls `begin_block` on all components.
    ///
    /// Returns the encoded upgrade change hashes if an upgrade was executed.
    ///
    /// This *must* be called any time before a block's txs are executed, whether it's
    /// during the proposal phase, or finalize_block phase.
    #[instrument(name = "App::pre_execute_transactions", skip_all, err(level = Level::WARN))]
    async fn pre_execute_transactions(&mut self, block_data: BlockData) -> Result<Vec<ChangeHash>> {
        let mut delta_delta = StateDelta::new(self.state.clone());
        let upgrade_change_hashes = self
            .upgrades_handler
            .execute_upgrade_if_due(&mut delta_delta, block_data.height)
            .await
            .wrap_err("failed to execute upgrade")?;
        if upgrade_change_hashes.is_empty() {
            // We need to drop this so there's only one reference to `self.state` left in order to
            // apply changes made in `self.begin_block()` below.
            drop(delta_delta);
        } else {
            let _ = self.apply(delta_delta);
        }

        let chain_id = self
            .state
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

        Ok(upgrade_change_hashes)
    }

    #[instrument(name = "App::extend_vote", skip_all)]
    pub(crate) async fn extend_vote(
        &mut self,
        _extend_vote: abci::request::ExtendVote,
    ) -> Result<abci::response::ExtendVote> {
        let start = Instant::now();
        let result = self.vote_extension_handler.extend_vote(&self.state).await;
        if result.is_ok() {
            self.metrics
                .record_extend_vote_duration_seconds(start.elapsed());
        } else {
            self.metrics.increment_extend_vote_failure_count();
        }
        result
    }

    #[instrument(name = "App::extend_vote", skip_all)]
    pub(crate) async fn verify_vote_extension(
        &mut self,
        vote_extension: abci::request::VerifyVoteExtension,
    ) -> Result<abci::response::VerifyVoteExtension> {
        let result = self
            .vote_extension_handler
            .verify_vote_extension(&self.state, vote_extension)
            .await;
        if result.is_err() {
            self.metrics.increment_verify_vote_extension_failure_count();
        }
        result
    }

    /// updates the app state after transaction execution, and generates the resulting
    /// `SequencerBlock`.
    ///
    /// this must be called after a block's transactions are executed.
    /// FIXME: don't return sequencer block but grab the block from state delta https://github.com/astriaorg/astria/issues/1436
    #[expect(clippy::too_many_arguments, reason = "should be refactored")]
    #[instrument(name = "App::post_execute_transactions", skip_all, err(level = Level::WARN))]
    async fn post_execute_transactions(
        &mut self,
        block_hash: Hash,
        height: tendermint::block::Height,
        time: tendermint::Time,
        proposer_address: account::Id,
        expanded_block_data: ExpandedBlockData,
        rollup_data_bytes: Vec<(RollupId, Bytes)>,
        tx_results: Vec<ExecTxResult>,
        executed_tx_ids: HashSet<TransactionId>,
    ) -> Result<SequencerBlock> {
        let Hash::Sha256(block_hash) = block_hash else {
            bail!("block hash is empty; this should not occur")
        };

        // Update the proposal fingerprint to include the full executed block data.
        self.execution_state
            .set_executed_block(block_hash)
            .wrap_err("failed to set executed proposal fingerprint, this should not happen")?;

        let chain_id = self
            .state
            .get_chain_id()
            .await
            .wrap_err("failed to get chain ID from state")?;
        let sudo_address = self
            .state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;

        let end_block = self.end_block(height.value(), &sudo_address).await?;

        // get deposits for this block from state's ephemeral cache and put them to storage.
        let mut state_tx = StateDelta::new(self.state.clone());
        let deposits_in_this_block = self.state.get_cached_block_deposits();
        debug!(deposits = %json(&deposits_in_this_block), "got block deposits from state");

        state_tx
            .put_deposits(&block_hash, deposits_in_this_block.clone())
            .wrap_err("failed to put deposits to state")?;

        // cometbft expects a result for every tx in the block, so we need to return a
        // tx result for the commitments and other injected data items, even though they're not
        // actually user txs.
        //
        // the tx_results passed to this function only contain results for every user-submitted
        // transaction, not the injected ones.
        let injected_tx_count = expanded_block_data.injected_transaction_count();
        let mut finalize_block_tx_results: Vec<ExecTxResult> =
            Vec::with_capacity(expanded_block_data.user_submitted_transactions.len());
        finalize_block_tx_results
            .extend(std::iter::repeat(ExecTxResult::default()).take(injected_tx_count));
        finalize_block_tx_results.extend(tx_results);

        let sequencer_block = SequencerBlockBuilder {
            block_hash: block::Hash::new(block_hash),
            chain_id,
            height,
            time,
            proposer_address,
            expanded_block_data,
            rollup_data_bytes,
            deposits: deposits_in_this_block,
        }
        .try_build()
        .wrap_err("failed to convert block info and data to SequencerBlock")?;
        state_tx
            .put_sequencer_block(sequencer_block.clone())
            .wrap_err("failed to write sequencer block to state")?;

        let consensus_param_updates = self
            .upgrades_handler
            .end_block(&mut state_tx, height)
            .await
            .wrap_err("upgrades handler failed to end block")?;

        if let Some(consensus_params) = &consensus_param_updates {
            info!(
                consensus_params = %display_consensus_params(consensus_params),
                "updated consensus params"
            );
        }

        let result = PostTransactionExecutionResult {
            events: end_block.events,
            validator_updates: end_block.validator_updates,
            tx_results: finalize_block_tx_results,
            consensus_param_updates,
            executed_tx_ids,
        };

        state_tx.object_put(POST_TRANSACTION_EXECUTION_RESULT_KEY, result);

        // events that occur after end_block are ignored here;
        // there should be none anyways.
        let _ = self.apply(state_tx);

        Ok(sequencer_block)
    }

    /// Executes the given block, but does not write it to disk.
    ///
    /// `commit` must be called after this to write the block to disk.
    ///
    /// This is called by cometbft after the block has already been
    /// committed by the network's consensus.
    #[instrument(name = "App::finalize_block", skip_all, err)]
    pub(crate) async fn finalize_block(
        &mut self,
        finalize_block: abci::request::FinalizeBlock,
        storage: Storage,
    ) -> Result<abci::response::FinalizeBlock> {
        let Hash::Sha256(block_hash) = finalize_block.hash else {
            bail!("block hash is empty; this should not occur")
        };
        // If there is not a matching cached executed proposal, we need to execute the block.
        let skip_execution = self.execution_state.check_if_executed_block(block_hash);
        if !skip_execution {
            // clear out state before execution.
            self.update_state_for_new_round(&storage);
        }

        let uses_data_item_enum = self.uses_data_item_enum(finalize_block.height);
        let expanded_block_data = if uses_data_item_enum {
            let with_extended_commit_info =
                self.vote_extensions_enabled(finalize_block.height).await?;
            ExpandedBlockData::new_from_typed_data(&finalize_block.txs, with_extended_commit_info)
        } else {
            ExpandedBlockData::new_from_untyped_data(&finalize_block.txs)
        }
        .wrap_err("failed to parse data items")?;

        let mut all_events = if let Some(extended_commit_info_with_proof) =
            &expanded_block_data.extended_commit_info_with_proof
        {
            let extended_commit_info = extended_commit_info_with_proof.extended_commit_info();
            self.metrics.record_extended_commit_info_bytes(
                extended_commit_info_with_proof
                    .encoded_extended_commit_info()
                    .len(),
            );
            let mut state_tx: StateDelta<Arc<StateDelta<Snapshot>>> =
                StateDelta::new(self.state.clone());
            vote_extension::apply_prices_from_vote_extensions(
                &mut state_tx,
                extended_commit_info,
                finalize_block.time.into(),
                finalize_block.height.value(),
            )
            .await
            .wrap_err("failed to apply prices from vote extensions")?;
            self.apply(state_tx)
        } else {
            vec![]
        };

        // FIXME: refactor to avoid cloning the finalize block
        let finalize_block_arc = Arc::new(finalize_block.clone());

        if !skip_execution {
            // we haven't executed anything yet, so set up the state for execution.
            let block_data = BlockData {
                misbehavior: finalize_block.misbehavior,
                height: finalize_block.height,
                time: finalize_block.time,
                next_validators_hash: finalize_block.next_validators_hash,
                proposer_address: finalize_block.proposer_address,
            };

            let upgrade_change_hashes = self
                .pre_execute_transactions(block_data)
                .await
                .wrap_err("failed to execute block")?;
            ensure_upgrade_change_hashes_as_expected(
                &expanded_block_data,
                upgrade_change_hashes.as_ref(),
            )?;

            let user_submitted_transactions = construct_checked_txs(
                &expanded_block_data.user_submitted_transactions,
                &self.state,
            )
            .await
            .wrap_err("failed to execute transactions in finalize block")?;
            let mut tx_results = Vec::with_capacity(user_submitted_transactions.len());
            let mut executed_tx_ids = HashSet::new();
            for tx in &user_submitted_transactions {
                match self.execute_transaction(tx.clone()).await {
                    Ok(events) => {
                        tx_results.push(ExecTxResult {
                            events,
                            ..Default::default()
                        });
                        executed_tx_ids.insert(*tx.id());
                    }
                    Err(error) => {
                        // this is actually a protocol error, as only valid txs should be finalized
                        tracing::error!(
                            %error,
                            "failed to finalize transaction; ignoring it",
                        );
                        let code = if matches!(
                            error,
                            CheckedTransactionExecutionError::InvalidNonce { .. }
                        ) {
                            AbciErrorCode::INVALID_NONCE
                        } else {
                            AbciErrorCode::INTERNAL_ERROR
                        };
                        tx_results.push(ExecTxResult {
                            code: Code::Err(code.value()),
                            info: code.info(),
                            log: format!("{:#}", Report::new(error)),
                            ..Default::default()
                        });
                    }
                }
            }
            let rollup_data_bytes = user_submitted_transactions
                .iter()
                .flat_map(|checked_tx| {
                    checked_tx
                        .rollup_data_bytes()
                        .map(|(rollup_id, data)| (*rollup_id, data.clone()))
                })
                .collect();

            self.post_execute_transactions(
                finalize_block.hash,
                finalize_block.height,
                finalize_block.time,
                finalize_block.proposer_address,
                expanded_block_data,
                rollup_data_bytes,
                tx_results,
                executed_tx_ids,
            )
            .await
            .wrap_err("failed to run post execute transactions handler")?;
        }

        let PostTransactionExecutionResult {
            events,
            tx_results,
            validator_updates,
            consensus_param_updates,
            executed_tx_ids,
        } = self
            .state
            .object_get(POST_TRANSACTION_EXECUTION_RESULT_KEY)
            .expect(
                "post_transaction_execution_result must be present, as txs were already executed \
                 just now or during the proposal phase",
            );

        // prepare the `WriteBatch` for a later commit.
        let app_hash = self
            .prepare_commit(storage, executed_tx_ids)
            .await
            .wrap_err("failed to prepare commit")?;
        all_events.extend(events);
        let finalize_block_response = abci::response::FinalizeBlock {
            events: all_events,
            tx_results,
            validator_updates,
            consensus_param_updates,
            app_hash,
        };

        self.event_bus.send_finalized_block(finalize_block_arc);

        Ok(finalize_block_response)
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn prepare_commit(
        &mut self,
        storage: Storage,
        executed_tx_ids: HashSet<TransactionId>,
    ) -> Result<AppHash> {
        // extract the state we've built up to so we can prepare it as a `StagedWriteBatch`.
        let dummy_state = StateDelta::new(storage.latest_snapshot());
        let mut state = Arc::try_unwrap(std::mem::replace(&mut self.state, Arc::new(dummy_state)))
            .expect("we have exclusive ownership of the State at commit()");

        // store the storage version indexed by block height
        let new_version = storage.latest_version().wrapping_add(1);
        let height = state
            .get_block_height()
            .await
            .expect("block height must be set, as `put_block_height` was already called");
        state
            .put_storage_version_by_height(height, new_version)
            .wrap_err("failed to put storage version by height")?;
        debug!(
            height,
            version = new_version,
            "stored storage version for height"
        );

        let write_batch = storage
            .prepare_commit(state)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to prepare commit")?;
        let app_hash: AppHash = write_batch
            .root_hash()
            .0
            .to_vec()
            .try_into()
            .wrap_err("failed to convert app hash")?;
        self.write_batch = Some(WriteBatch {
            write_batch,
            executed_tx_ids,
        });
        Ok(app_hash)
    }

    #[instrument(name = "App::begin_block", skip_all, err(level = Level::WARN))]
    async fn begin_block(
        &mut self,
        begin_block: &abci::request::BeginBlock,
    ) -> Result<Vec<abci::Event>> {
        let mut state_tx = StateDelta::new(self.state.clone());

        state_tx
            .put_block_height(begin_block.header.height.into())
            .wrap_err("failed to put block height")?;
        state_tx
            .put_block_timestamp(begin_block.header.time)
            .wrap_err("failed to put block timestamp")?;

        // call begin_block on all components
        let mut arc_state_tx = Arc::new(state_tx);
        AccountsComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on AccountsComponent")?;
        AuthorityComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on AuthorityComponent")?;
        IbcComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on IbcComponent")?;
        FeesComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on FeesComponent")?;

        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        Ok(self.apply(state_tx))
    }

    /// Executes a checked transaction.
    #[instrument(name = "App::execute_transaction", skip_all, err(level = Level::DEBUG))]
    async fn execute_transaction(
        &mut self,
        tx: Arc<CheckedTransaction>,
    ) -> std::result::Result<Vec<Event>, CheckedTransactionExecutionError> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should be present and unique");

        tx.execute(&mut state_tx).await?;

        // flag mempool for cleaning if we ran a fee change action
        let changes_fees = |action: &CheckedAction| {
            matches!(
                action,
                CheckedAction::FeeChange(_) | CheckedAction::FeeAssetChange(_)
            )
        };
        self.recost_mempool = self.recost_mempool || tx.checked_actions().iter().any(changes_fees);
        // index all event attributes
        let mut events = state_tx.apply().1;
        for event in &mut events {
            event
                .attributes
                .iter_mut()
                .for_each(|attr| attr.set_index(true));
        }

        Ok(events)
    }

    #[instrument(name = "App::end_block", skip_all, err(level = Level::WARN))]
    async fn end_block(
        &mut self,
        height: u64,
        fee_recipient: &[u8; 20],
    ) -> Result<abci::response::EndBlock> {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        let end_block = abci::request::EndBlock {
            height: height
                .try_into()
                .expect("a block height should be able to fit in an i64"),
        };

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on AccountsComponent")?;
        AuthorityComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on AuthorityComponent")?;
        FeesComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on FeesComponent")?;
        IbcComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on IbcComponent")?;

        let mut state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        // gather and return validator updates
        let validator_updates = self
            .state
            .get_block_validator_updates()
            .await
            .expect("failed getting validator updates");

        // clear validator updates
        state_tx.clear_block_validator_updates();

        // gather block fees and transfer them to the fee recipient
        let block_fees = self.state.get_block_fees();

        for (fee_asset, total_amount) in block_fees {
            state_tx
                .increase_balance(fee_recipient, &fee_asset, total_amount)
                .await
                .wrap_err("failed to increase fee recipient balance")?;
        }

        let events = self.apply(state_tx);
        Ok(abci::response::EndBlock {
            validator_updates: validator_updates
                .try_into_cometbft()
                .wrap_err("failed converting astria validators to cometbft compatible type")?,
            events,
            ..Default::default()
        })
    }

    #[instrument(name = "App::commit", skip_all)]
    pub(crate) async fn commit(&mut self, storage: Storage) -> Result<ShouldShutDown> {
        let WriteBatch {
            write_batch,
            executed_tx_ids,
        } = self.write_batch.take().expect(
            "write batch must be set, as `finalize_block` is always called before `commit`",
        );

        // Commit the pending writes, clearing the state.
        let app_hash = storage
            .commit_batch(write_batch)
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
        // and clear the previous fingerprint.
        self.update_state_for_new_round(&storage);

        // update the priority of any txs in the mempool based on the updated app state
        if self.recost_mempool {
            self.metrics.increment_mempool_recosted();
        }

        let block_height = self
            .state
            .get_block_height()
            .await
            .expect("block height must exist in state");

        update_mempool_after_finalization(
            &mut self.mempool,
            &self.state,
            self.recost_mempool,
            &executed_tx_ids,
            block_height,
        )
        .await;

        self.upgrades_handler
            .should_shut_down(&storage.latest_snapshot())
            .await
    }

    // StateDelta::apply only works when the StateDelta wraps an underlying
    // StateWrite.  But if we want to share the StateDelta with spawned tasks,
    // we usually can't wrap a StateWrite instance, which requires exclusive
    // access. This method "externally" applies the state delta to the
    // inter-block state.
    //
    // Invariant: state_tx and self.state are the only two references to the
    // inter-block state.
    fn apply(&mut self, state_tx: StateDelta<InterBlockState>) -> Vec<Event> {
        let (state2, mut cache) = state_tx.flatten();
        std::mem::drop(state2);
        // Now there is only one reference to the inter-block state: self.state

        let events = cache.take_events();
        cache.apply_to(
            Arc::get_mut(&mut self.state).expect("no other references to inter-block state"),
        );

        events
    }

    /// Returns whether or not the block at the given height uses encoded `DataItem`s as the raw
    /// `txs` field of `response::PrepareProposal`, and hence also `request::ProcessProposal` and
    /// `request::FinalizeBlock`.
    ///
    /// This behavior was introduced in `Aspen`.  If `Aspen` is not included in the upgrades
    /// files, the assumption is that this network does not use `DataItem`s from genesis onwards.
    ///
    /// Returns `true` if and only if `Aspen` is in upgrades and `block_height` is greater than or
    /// equal to its activation height.
    fn uses_data_item_enum(&self, block_height: tendermint::block::Height) -> bool {
        self.upgrades_handler
            .upgrades()
            .aspen()
            .map_or(false, |aspen| {
                block_height.value() >= aspen.activation_height()
            })
    }

    /// Returns `true` if vote extensions are enabled for the block at the given height, i.e. if
    /// `block_height` is greater than `vote_extensions_enable_height` of the stored consensus
    /// params, and `vote_extensions_enable_height` is not 0.
    ///
    /// NOTE: This returns `false` if `block_height` is EQUAL TO `vote_extensions_enable_height`
    ///       since it takes one block for the extended votes to become available for voting on in
    ///       the next block.
    async fn vote_extensions_enabled(
        &mut self,
        block_height: tendermint::block::Height,
    ) -> Result<bool> {
        let vote_extensions_enable_height = self
            .state
            .get_consensus_params()
            .await
            .wrap_err("failed to get consensus params from storage")?
            .map_or(0, |consensus_params| {
                vote_extensions_enable_height(&consensus_params)
            });
        // NOTE: if the value of `vote_extensions_enable_height` is zero, vote extensions are
        // disabled. See
        // https://docs.cometbft.com/v0.38/spec/abci/abci++_app_requirements#abciparamsvoteextensionsenableheight
        Ok(
            vote_extensions_enable_height != VOTE_EXTENSIONS_DISABLED_HEIGHT
                && block_height.value() > vote_extensions_enable_height,
        )
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn mempool(&self) -> Mempool {
        self.mempool.clone()
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn upgrades_handler(&self) -> &UpgradesHandler {
        &self.upgrades_handler
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn state(&self) -> &StateDelta<Snapshot> {
        &self.state
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn state_mut(&mut self) -> &mut StateDelta<Snapshot> {
        Arc::get_mut(&mut self.state).unwrap()
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn new_state_delta(&self) -> StateDelta<Arc<StateDelta<Snapshot>>> {
        StateDelta::new(self.state.clone())
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn metrics(&self) -> &'static Metrics {
        self.metrics
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) fn into_events(self) -> Vec<Event> {
        Arc::into_inner(self.state)
            .unwrap()
            .flatten()
            .1
            .take_events()
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) async fn authority_component_end_block(&mut self) {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);
        let end_block = abci::request::EndBlock {
            height: 1,
        };
        AuthorityComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .unwrap();
        let state_tx = Arc::try_unwrap(arc_state_tx).unwrap();
        let _ = self.apply(state_tx);
    }

    #[cfg(any(test, feature = "benchmark"))]
    pub(crate) async fn apply_and_commit(
        &mut self,
        state_delta: StateDelta<InterBlockState>,
        storage: Storage,
    ) {
        let _events = self.apply(state_delta);
        self.prepare_commit(storage.clone(), HashSet::new())
            .await
            .unwrap();
        self.commit(storage).await.unwrap();
    }
}

fn vote_extensions_enable_height(consensus_params: &tendermint::consensus::Params) -> u64 {
    consensus_params
        .abci
        .vote_extensions_enable_height
        .map_or(0, |height| height.value())
}

fn ensure_upgrade_change_hashes_as_expected(
    received_data: &ExpandedBlockData,
    calculated_upgrade_change_hashes: &[ChangeHash],
) -> Result<()> {
    ensure!(
        received_data.upgrade_change_hashes == calculated_upgrade_change_hashes,
        "upgrade change hashes ({:?}) do not match expected ({calculated_upgrade_change_hashes:?})",
        received_data.upgrade_change_hashes
    );
    Ok(())
}

pub(crate) enum ShouldShutDown {
    ShutDownForUpgrade {
        upgrade_activation_height: u64,
        block_time: Time,
        hex_encoded_app_hash: String,
    },
    ContinueRunning,
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
    txs_included_in_block: &HashSet<TransactionId>,
    block_height: u64,
) {
    mempool
        .run_maintenance(state, recost, txs_included_in_block, block_height)
        .await;
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

#[derive(Clone, Debug)]
struct PostTransactionExecutionResult {
    events: Vec<Event>,
    tx_results: Vec<ExecTxResult>,
    validator_updates: Vec<tendermint::validator::Update>,
    consensus_param_updates: Option<tendermint::consensus::Params>,
    executed_tx_ids: HashSet<TransactionId>,
}

#[derive(PartialEq)]
enum BreakOrContinue {
    Break,
    Continue,
}

impl BreakOrContinue {
    fn should_break(self) -> bool {
        match self {
            BreakOrContinue::Break => true,
            BreakOrContinue::Continue => false,
        }
    }
}

enum Proposal {
    Prepare {
        block_size_constraints: BlockSizeConstraints,
        included_txs: Vec<Arc<CheckedTransaction>>,
        failed_tx_count: usize,
        execution_results: Vec<ExecTxResult>,
        executed_tx_ids: HashSet<TransactionId>,
        excluded_tx_count: usize,
        current_tx_group: Group,
        mempool: Mempool,
        metrics: &'static Metrics,
    },
    Process {
        block_size_constraints: BlockSizeConstraints,
        execution_results: Vec<ExecTxResult>,
        executed_tx_ids: HashSet<TransactionId>,
        current_tx_group: Group,
    },
}

impl Proposal {
    fn block_size_constraints(&self) -> &BlockSizeConstraints {
        match self {
            Proposal::Prepare {
                block_size_constraints,
                ..
            }
            | Proposal::Process {
                block_size_constraints,
                ..
            } => block_size_constraints,
        }
    }

    fn block_size_constraints_mut(&mut self) -> &mut BlockSizeConstraints {
        match self {
            Proposal::Prepare {
                block_size_constraints,
                ..
            }
            | Proposal::Process {
                block_size_constraints,
                ..
            } => block_size_constraints,
        }
    }

    fn current_tx_group(&self) -> Group {
        match self {
            Proposal::Prepare {
                current_tx_group, ..
            }
            | Proposal::Process {
                current_tx_group, ..
            } => *current_tx_group,
        }
    }

    fn set_current_tx_group(&mut self, group: Group) {
        match self {
            Proposal::Prepare {
                current_tx_group, ..
            }
            | Proposal::Process {
                current_tx_group, ..
            } => *current_tx_group = group,
        }
    }

    fn execution_results_and_tx_ids(self) -> (Vec<ExecTxResult>, HashSet<TransactionId>) {
        match self {
            Proposal::Prepare {
                execution_results,
                executed_tx_ids,
                ..
            }
            | Proposal::Process {
                execution_results,
                executed_tx_ids,
                ..
            } => (execution_results, executed_tx_ids),
        }
    }

    fn execution_results_and_tx_ids_mut(
        &mut self,
    ) -> (&mut Vec<ExecTxResult>, &mut HashSet<TransactionId>) {
        match self {
            Proposal::Prepare {
                execution_results,
                executed_tx_ids,
                ..
            }
            | Proposal::Process {
                execution_results,
                executed_tx_ids,
                ..
            } => (execution_results, executed_tx_ids),
        }
    }
}

async fn construct_checked_txs<S: StateRead>(
    encoded_txs: &[Bytes],
    state: &S,
) -> Result<Vec<Arc<CheckedTransaction>>> {
    let txs_futures = encoded_txs.iter().map(|encoded_tx| async move {
        let tx = CheckedTransaction::new(encoded_tx.clone(), state)
            .await
            .wrap_err("failed to construct checked transaction")?;
        Ok(Arc::new(tx))
    });

    try_join_all(txs_futures).await
}

fn display_consensus_params(params: &tendermint::consensus::Params) -> String {
    let unset = || "unset".to_string();
    format!(
        "block.max_bytes: {}, block.max_gas: {}, block.time_iota_ms: {}, \
         evidence.max_age_num_blocks: {}, evidence.max_age_duration: {:?}, evidence.max_bytes: \
         {}, validator.pub_key_types: {:?}, version.app: {}, abci.vote_extensions_enable_height: \
         {}",
        params.block.max_bytes,
        params.block.max_gas,
        params.block.time_iota_ms,
        params.evidence.max_age_num_blocks,
        params.evidence.max_age_duration.0,
        params.evidence.max_bytes,
        params.validator.pub_key_types,
        params
            .version
            .as_ref()
            .map_or_else(unset, |version_params| version_params.app.to_string()),
        params
            .abci
            .vote_extensions_enable_height
            .map_or_else(unset, |height| height.to_string()),
    )
}
