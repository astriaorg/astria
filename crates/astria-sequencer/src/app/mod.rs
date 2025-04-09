#[cfg(any(test, feature = "benchmark"))]
pub(crate) mod benchmark_and_test_utils;
#[cfg(feature = "benchmark")]
mod benchmarks;
pub(crate) mod event_bus;
mod execution_state;
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
    generated::astria::protocol::transaction::v1 as raw,
    primitive::v1::TRANSACTION_ID_LEN,
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
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        ensure,
        eyre,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    ArcStateDeltaExt,
    Snapshot,
    StagedWriteBatch,
    StateDelta,
    StateRead,
    StateWrite,
    Storage,
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
    trace,
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
    action_handler::{
        impls::transaction::InvalidNonce,
        ActionHandler as _,
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
};

// ephemeral store key for the cache of results of executing of transactions in `prepare_proposal`.
// cleared in `process_proposal` if we're the proposer.
const EXECUTION_RESULTS_KEY: &str = "execution_results";

// ephemeral store key for the cache of results of executing of transactions in `process_proposal`.
// cleared at the end of the block.
const POST_TRANSACTION_EXECUTION_RESULT_KEY: &str = "post_transaction_execution_result";

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

    // the sequencer event bus, used to send and receive events between components within the app
    event_bus: EventBus,

    metrics: &'static Metrics,
}

impl App {
    #[instrument(name = "App::new", skip_all, err)]
    pub(crate) async fn new(
        snapshot: Snapshot,
        mempool: Mempool,
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

        let mut block_size_constraints = BlockSizeConstraints::new(
            usize::try_from(prepare_proposal.max_tx_bytes)
                .wrap_err("failed to convert max_tx_bytes to usize")?,
        )
        .wrap_err("failed to create block size constraints")?;

        let request = prepare_proposal.clone();
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
            .prepare_proposal_tx_execution(&mut block_size_constraints)
            .await
            .wrap_err("failed to execute transactions")?;
        self.metrics
            .record_proposal_transactions(signed_txs_included.len());

        let deposits = self.state.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        // generate commitment to sequence::Actions and deposits and commitment to the rollup IDs
        // included in the block
        let res = generate_rollup_datas_commitment(&signed_txs_included, deposits);
        let txs = res.into_transactions(included_tx_bytes);

        let response = abci::response::PrepareProposal {
            txs,
        };
        // Generate the prepared proposal fingerprint.
        self.execution_state
            .set_prepared_proposal(request.clone(), response.clone())
            .wrap_err("failed to set executed proposal fingerprint, this should not happen")?;
        Ok(response)
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions
    /// and ensures it matches the commitment created by the proposer, which
    /// should be the first transaction in the block.
    #[instrument(name = "App::process_proposal", skip_all, err(level = Level::WARN))]
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
                self.update_state_for_new_round(&storage);
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

        // If we can skip execution just fetch the cache, otherwise need to run the execution.
        let tx_results = if skip_execution {
            // if we're the proposer, we should have the execution results from
            // `prepare_proposal`. run the post-tx-execution hook to generate the
            // `SequencerBlock` and to set `self.finalize_block`.
            //
            // we can't run this in `prepare_proposal` as we don't know the block hash there.
            let Some(tx_results) = self.state.object_get(EXECUTION_RESULTS_KEY) else {
                bail!("execution results must be present after executing transactions")
            };

            tx_results
        } else {
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
                misbehavior: process_proposal.misbehavior.clone(),
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
            // this does not error if any txs fail to be deserialized, but the
            // `execution_results.len()` check below ensures that all txs in the
            // proposal are deserializable (and executable).
            let signed_txs = txs
                .into_iter()
                .filter_map(|bytes| signed_transaction_from_bytes(bytes.as_ref()).ok())
                .collect::<Vec<_>>();

            let tx_results = self
                .process_proposal_tx_execution(signed_txs.clone(), &mut block_size_constraints)
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

            let deposits = self.state.get_cached_block_deposits();
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

            tx_results
        };

        let sequencer_block = self
            .post_execute_transactions(
                process_proposal.hash,
                process_proposal.height,
                process_proposal.time,
                process_proposal.proposer_address,
                process_proposal.txs,
                tx_results,
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
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<(Vec<bytes::Bytes>, Vec<Transaction>)> {
        let mempool_len = self.mempool.len().await;
        debug!(mempool_len, "executing transactions from mempool");

        let mut proposal_info = Proposal::Prepare {
            validated_txs: Vec::new(),
            included_signed_txs: Vec::new(),
            failed_tx_count: 0,
            execution_results: Vec::new(),
            excluded_txs: 0,
            current_tx_group: Group::BundleableGeneral,
            mempool: self.mempool.clone(),
            metrics: self.metrics,
        };

        // get copy of transactions to execute from mempool
        let pending_txs = self.mempool.builder_queue().await;

        let mut unused_count = pending_txs.len();
        for (tx_hash, tx) in pending_txs {
            unused_count = unused_count.saturating_sub(1);

            if BreakOrContinue::Break
                == proposal_checks_and_tx_execution(
                    self,
                    tx,
                    Some(tx_hash),
                    block_size_constraints,
                    &mut proposal_info,
                )
                .await?
            {
                break;
            }
        }

        let Proposal::Prepare {
            validated_txs,
            included_signed_txs,
            failed_tx_count,
            execution_results,
            excluded_txs,
            ..
        } = proposal_info
        else {
            bail!("expected `Proposal::Prepare`, received `Proposal::Process`")
        };

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
        let mut state_tx = Arc::try_begin_transaction(&mut self.state)
            .expect("state Arc should not be referenced elsewhere");
        state_tx.object_put(EXECUTION_RESULTS_KEY, execution_results);
        let _ = state_tx.apply();

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
    #[instrument(name = "App::process_proposal_tx_execution", skip_all, err(level = Level::DEBUG))]
    async fn process_proposal_tx_execution(
        &mut self,
        txs: Vec<Transaction>,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<Vec<ExecTxResult>> {
        let mut proposal_info = Proposal::Process {
            execution_results: vec![],
            current_tx_group: Group::BundleableGeneral,
        };

        for tx in txs {
            let tx = Arc::new(tx);
            if BreakOrContinue::Break
                == proposal_checks_and_tx_execution(
                    self,
                    tx,
                    None,
                    block_size_constraints,
                    &mut proposal_info,
                )
                .await?
            {
                break;
            }
        }
        Ok(proposal_info.execution_results())
    }

    /// sets up the state for execution of the block's transactions.
    /// set the current height and timestamp, and calls `begin_block` on all components.
    ///
    /// this *must* be called anytime before a block's txs are executed, whether it's
    /// during the proposal phase, or finalize_block phase.
    #[instrument(name = "App::pre_execute_transactions", skip_all, err(level = Level::WARN))]
    async fn pre_execute_transactions(&mut self, block_data: BlockData) -> Result<()> {
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

        Ok(())
    }

    /// updates the app state after transaction execution, and generates the resulting
    /// `SequencerBlock`.
    ///
    /// this must be called after a block's transactions are executed.
    /// FIXME: don't return sequencer block but grab the block from state delta https://github.com/astriaorg/astria/issues/1436
    #[instrument(name = "App::post_execute_transactions", skip_all, err(level = Level::WARN))]
    async fn post_execute_transactions(
        &mut self,
        block_hash: Hash,
        height: tendermint::block::Height,
        time: tendermint::Time,
        proposer_address: account::Id,
        txs: Vec<bytes::Bytes>,
        tx_results: Vec<ExecTxResult>,
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
        debug!(
            deposits = %telemetry::display::json(&deposits_in_this_block),
            "got block deposits from state"
        );

        state_tx
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
        state_tx
            .put_sequencer_block(sequencer_block.clone())
            .wrap_err("failed to write sequencer block to state")?;

        let result = PostTransactionExecutionResult {
            events: end_block.events,
            validator_updates: end_block.validator_updates,
            consensus_param_updates: end_block.consensus_param_updates,
            tx_results: finalize_block_tx_results,
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
        ensure!(
            finalize_block.txs.len() >= 2,
            "block must contain at least two transactions: the rollup transactions commitment and
             rollup IDs commitment"
        );

        // FIXME: refactor to avoid cloning the finalize block
        let finalize_block_arc = Arc::new(finalize_block.clone());

        // We only need to do execution if we haven't executed the proposal yet.
        let Hash::Sha256(block_hash) = finalize_block.hash else {
            bail!("block hash is empty; this should not occur")
        };

        // If there is not a matching cached executed proposal, we need to execute the block.
        if !self.execution_state.check_if_executed_block(block_hash) {
            // clear out state before execution.
            self.update_state_for_new_round(&storage);
            // convert tendermint id to astria address; this assumes they are
            // the same address, as they are both ed25519 keys
            let proposer_address = finalize_block.proposer_address;
            let time = finalize_block.time;

            // we haven't executed anything yet, so set up the state for execution.
            let block_data = BlockData {
                misbehavior: finalize_block.misbehavior,
                height: finalize_block.height,
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
                finalize_block.height,
                time,
                proposer_address,
                finalize_block.txs,
                tx_results,
            )
            .await
            .wrap_err("failed to run post execute transactions handler")?;
        }

        let post_transaction_execution_result: PostTransactionExecutionResult = self
            .state
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
        let finalize_block_response = abci::response::FinalizeBlock {
            events: post_transaction_execution_result.events,
            validator_updates: post_transaction_execution_result.validator_updates,
            consensus_param_updates: post_transaction_execution_result.consensus_param_updates,
            app_hash,
            tx_results: post_transaction_execution_result.tx_results,
        };

        self.event_bus.send_finalized_block(finalize_block_arc);

        Ok(finalize_block_response)
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn prepare_commit(&mut self, storage: Storage) -> Result<AppHash> {
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
        self.write_batch = Some(write_batch);
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

    /// Executes a signed transaction.
    #[instrument(name = "App::execute_transaction", skip_all, err(level = Level::DEBUG))]
    async fn execute_transaction(&mut self, signed_tx: Arc<Transaction>) -> Result<Vec<Event>> {
        signed_tx
            .check_stateless()
            .await
            .wrap_err("stateless check failed")?;

        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should be present and unique");

        signed_tx
            .check_and_execute(&mut state_tx)
            .await
            .wrap_err("failed executing transaction")?;

        // flag mempool for cleaning if we ran a fee change action
        self.recost_mempool = self.recost_mempool
            || signed_tx.is_bundleable_sudo_action_group()
                && signed_tx
                    .actions()
                    .iter()
                    .any(|act| act.is_fee_asset_change() || act.is_fee_change());

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
            .get_validator_updates()
            .await
            .expect("failed getting validator updates");

        // clear validator updates
        state_tx.clear_validator_updates();

        // gather block fees and transfer them to the block proposer
        let fees = self.state.get_block_fees();

        for fee in fees {
            state_tx
                .increase_balance(fee_recipient, fee.asset(), fee.amount())
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
        // and clear the previous fingerprint.
        self.update_state_for_new_round(&storage);

        // update the priority of any txs in the mempool based on the updated app state
        if self.recost_mempool {
            self.metrics.increment_mempool_recosted();
        }
        update_mempool_after_finalization(&mut self.mempool, &self.state, self.recost_mempool)
            .await;
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

#[derive(PartialEq)]
enum BreakOrContinue {
    Break,
    Continue,
}

enum Proposal {
    Prepare {
        validated_txs: Vec<bytes::Bytes>,
        included_signed_txs: Vec<Transaction>,
        failed_tx_count: usize,
        execution_results: Vec<ExecTxResult>,
        excluded_txs: usize,
        current_tx_group: Group,
        mempool: Mempool,
        metrics: &'static Metrics,
    },
    Process {
        execution_results: Vec<ExecTxResult>,
        current_tx_group: Group,
    },
}

impl Proposal {
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

    fn execution_results_mut(&mut self) -> &mut Vec<ExecTxResult> {
        match self {
            Proposal::Prepare {
                execution_results, ..
            }
            | Proposal::Process {
                execution_results, ..
            } => execution_results,
        }
    }

    fn execution_results(self) -> Vec<ExecTxResult> {
        match self {
            Proposal::Prepare {
                execution_results, ..
            }
            | Proposal::Process {
                execution_results, ..
            } => execution_results,
        }
    }
}

#[instrument(skip_all)]
async fn proposal_checks_and_tx_execution(
    app: &mut App,
    tx: Arc<Transaction>,
    // `prepare_proposal_tx_execution` already has the tx hash, so we pass it in here
    tx_hash: Option<[u8; TRANSACTION_ID_LEN]>,
    block_size_constraints: &mut BlockSizeConstraints,
    proposal_info: &mut Proposal,
) -> Result<BreakOrContinue> {
    let tx_bytes = tx.to_raw().encode_to_vec();
    let tx_hash_base_64 =
        telemetry::display::base64(tx_hash.unwrap_or_else(|| Sha256::digest(&tx_bytes).into()))
            .to_string();
    let tx_len = tx_bytes.len();

    info!(transaction_hash = %tx_hash_base_64, "executing transaction");

    // check CometBFT size constraints for `prepare_proposal`
    if let Proposal::Prepare {
        metrics,
        excluded_txs,
        ..
    } = proposal_info
    {
        if !block_size_constraints.cometbft_has_space(tx_len) {
            metrics.increment_prepare_proposal_excluded_transactions_cometbft_space();
            debug!(
                transaction_hash = %tx_hash_base_64,
                block_size_constraints = %json(&block_size_constraints),
                tx_data_bytes = tx_len,
                "excluding remaining transactions: max cometBFT data limit reached"
            );
            *excluded_txs = excluded_txs.saturating_add(1);

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
    let tx_sequence_data_bytes = tx
        .unsigned_transaction()
        .actions()
        .iter()
        .filter_map(Action::as_rollup_data_submission)
        .fold(0usize, |acc, seq| acc.saturating_add(seq.data.len()));
    if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
        debug!(
            transaction_hash = %tx_hash_base_64,
            block_size_constraints = %json(&block_size_constraints),
            tx_data_bytes = tx_sequence_data_bytes,
            "{debug_msg}: max block sequenced data limit reached"
        );
        match proposal_info {
            Proposal::Prepare {
                metrics,
                excluded_txs,
                ..
            } => {
                metrics.increment_prepare_proposal_excluded_transactions_sequencer_space();
                *excluded_txs = excluded_txs.saturating_add(1);

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
            transaction_hash = %tx_hash_base_64,
            "{debug_msg}: group is higher priority than previously included transactions"
        );
        match proposal_info {
            Proposal::Prepare {
                excluded_txs, ..
            } => {
                *excluded_txs = excluded_txs.saturating_add(1);
                return Ok(BreakOrContinue::Continue);
            }
            Proposal::Process {
                ..
            } => {
                bail!("transactions have incorrect transaction group ordering");
            }
        };
    }

    let execution_results = proposal_info.execution_results_mut();
    match app.execute_transaction(tx.clone()).await {
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
            if let Proposal::Prepare {
                validated_txs,
                included_signed_txs,
                ..
            } = proposal_info
            {
                validated_txs.push(tx_bytes.into());
                included_signed_txs.push((*tx).clone());
            }
        }
        Err(e) => {
            debug!(
                transaction_hash = %tx_hash_base_64,
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
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
                        *failed_tx_count = failed_tx_count.saturating_add(1);

                        // remove the failing transaction from the mempool
                        //
                        // this will remove any transactions from the same sender
                        // as well, as the dependent nonces will not be able
                        // to execute
                        mempool
                            .remove_tx_invalid(
                                tx,
                                RemovalReason::FailedPrepareProposal(e.to_string()),
                            )
                            .await;
                    }
                }
                Proposal::Process {
                    ..
                } => return Err(e.wrap_err("transaction failed to execute")),
            }
        }
    };
    proposal_info.set_current_tx_group(tx_group);
    Ok(BreakOrContinue::Continue)
}
