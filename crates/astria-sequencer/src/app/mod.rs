mod action_handler;
pub(crate) mod app_abci;
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

use std::sync::Arc;

use astria_core::{
    protocol::transaction::v1::{
        action::group::Group,
        Action,
        Transaction,
    },
    sequencerblock::v1::block::SequencerBlock,
    Protobuf as _,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        eyre,
        OptionExt,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    ArcStateDeltaExt,
    Snapshot,
    StagedWriteBatch,
    StateDelta,
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
        Event,
    },
    account,
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
    accounts::StateWriteExt as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    fees::StateReadExt as _,
    grpc::StateWriteExt as _,
    mempool::{
        Mempool,
        RemovalReason,
    },
    metrics::Metrics,
    proposal::block_size_constraints::BlockSizeConstraints,
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
    state: InterBlockState,

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
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get current root hash")?
            .0
            .to_vec()
            .try_into()
            .expect("root hash conversion must succeed; should be 32 bytes");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Ok(Self {
            state,
            mempool,
            executed_proposal_fingerprint: None,
            executed_proposal_hash: Hash::default(),
            recost_mempool: false,
            write_batch: None,
            app_hash,
            metrics,
        })
    }

    #[instrument(skip_all, err)]
    pub(crate) async fn prepare_commit(&mut self, storage: Storage) -> Result<AppHash> {
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

    /// Executes a signed transaction.
    #[instrument(name = "App::execute_transaction", skip_all)]
    pub(crate) async fn execute_transaction(
        &mut self,
        signed_tx: Arc<Transaction>,
    ) -> Result<Vec<Event>> {
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

        Ok(state_tx.apply().1)
    }

    /// sets up the state for execution of the block's transactions.
    /// set the current height and timestamp, and removes byzantine validators from state.
    ///
    /// this *must* be called anytime before a block's txs are executed, whether it's
    /// during the proposal phase, or finalize_block phase.
    #[instrument(name = "App::prepare_state_for_execution", skip_all, err)]
    pub(crate) async fn prepare_state_for_execution(
        &mut self,
        block_data: BlockData,
    ) -> Result<()> {
        // reset recost flag
        self.recost_mempool = false;

        let mut state_tx = StateDelta::new(self.state.clone());

        state_tx
            .put_block_height(block_data.height.into())
            .wrap_err("failed to put block height")?;
        state_tx
            .put_block_timestamp(block_data.time)
            .wrap_err("failed to put block timestamp")?;

        let mut current_set = state_tx
            .get_validator_set()
            .await
            .wrap_err("failed getting validator set")?;

        for misbehaviour in &block_data.misbehavior {
            current_set.remove(&misbehaviour.validator.address);
        }

        state_tx
            .put_validator_set(current_set)
            .wrap_err("failed putting validator set")?;

        self.apply(state_tx);

        Ok(())
    }

    /// updates the app state after transaction execution, and generates the resulting
    /// `SequencerBlock`.
    ///
    /// this must be called after a block's transactions are executed.
    #[instrument(name = "App::update_state_and_end_block", skip_all)]
    pub(crate) async fn update_state_and_end_block(
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
            .state
            .get_chain_id()
            .await
            .wrap_err("failed to get chain ID from state")?;
        let sudo_address = self
            .state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;

        let mut state_tx = StateDelta::new(self.state.clone());

        // get validator updates
        let validator_updates = state_tx
            .get_validator_updates()
            .await
            .wrap_err("failed getting validator updates")?;

        // update validator set
        let mut current_set = state_tx
            .get_validator_set()
            .await
            .wrap_err("failed getting validator set")?;
        current_set.apply_updates(validator_updates.clone());
        state_tx
            .put_validator_set(current_set)
            .wrap_err("failed putting validator set")?;

        // clear validator updates
        state_tx.clear_validator_updates();

        // gather block fees and transfer them to the sudo address
        let fees = self.state.get_block_fees();
        for fee in fees {
            state_tx
                .increase_balance(&sudo_address, fee.asset(), fee.amount())
                .await
                .wrap_err("failed to increase fee recipient balance")?;
        }

        // get deposits for this block from state's ephemeral cache and put them to storage.
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
            .put_sequencer_block(sequencer_block)
            .wrap_err("failed to write sequencer block to state")?;

        let events = self.apply(state_tx);

        let mut state_tx = StateDelta::new(self.state.clone());
        let result = PostTransactionExecutionResult {
            events,
            validator_updates: validator_updates
                .try_into_cometbft()
                .wrap_err("failed converting astria validators to cometbft compatible type")?,
            tx_results: finalize_block_tx_results,
        };

        state_tx.object_put(POST_TRANSACTION_EXECUTION_RESULT_KEY, result);
        let _ = self.apply(state_tx);

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
    #[instrument(name = "App::prepare_proposal_tx_execution", skip_all)]
    async fn prepare_proposal_tx_execution(
        &mut self,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<(Vec<bytes::Bytes>, Vec<Transaction>)> {
        let mempool_len = self.mempool.len().await;
        debug!(mempool_len, "executing transactions from mempool");

        let mut proposal_info = Proposal::Prepare(PrepareProposalInformation::new(
            self.mempool.clone(),
            self.metrics,
        ));

        // get copy of transactions to execute from mempool
        let pending_txs = self
            .mempool
            .builder_queue(&self.state)
            .await
            .expect("failed to fetch pending transactions");

        let mut unused_count = pending_txs.len();
        for (tx_hash, tx) in pending_txs {
            unused_count = unused_count.saturating_sub(1);
            let tx_hash_base_64 = telemetry::display::base64(&tx_hash).to_string();
            info!(transaction_hash = %tx_hash_base_64, "executing transaction");

            proposal_checks_and_tx_execution(
                self,
                tx,
                &tx_hash_base_64,
                block_size_constraints,
                &mut proposal_info,
            )
            .await?;
        }

        let PrepareProposalInformation {
            validated_txs,
            included_signed_txs,
            failed_tx_count,
            execution_results,
            excluded_txs,
            ..
        } = proposal_info
            .into_prepare()
            .ok_or_eyre("expected `Proposal::Prepare`, received `Proposal::Process`")?;

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
    #[instrument(name = "App::process_proposal_tx_execution", skip_all)]
    async fn process_proposal_tx_execution(
        &mut self,
        txs: Vec<Transaction>,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> Result<Vec<ExecTxResult>> {
        let mut proposal_info = Proposal::Process(ProcessProposalInformation::new());

        for tx in txs {
            let tx = Arc::new(tx);
            let bytes = tx.to_raw().encode_to_vec();
            let tx_hash = Sha256::digest(&bytes);
            let tx_hash_base_64 = telemetry::display::base64(&tx_hash).to_string();

            proposal_checks_and_tx_execution(
                self,
                tx,
                &tx_hash_base_64,
                block_size_constraints,
                &mut proposal_info,
            )
            .await?;
        }

        let ProcessProposalInformation {
            execution_results, ..
        } = proposal_info
            .into_process()
            .ok_or_eyre("expected `Proposal::Process`, received `Proposal::Prepare`")?;

        Ok(execution_results)
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

    fn update_state_for_new_round(&mut self, storage: &Storage) {
        // reset app state to latest committed state, in case of a round not being committed
        // but `self.state` was changed due to executing the previous round's data.
        //
        // if the previous round was committed, then the state stays the same.
        //
        // this also clears the ephemeral storage.
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        // clear the cached executed proposal hash
        self.executed_proposal_hash = Hash::default();
    }
}

/// relevant data of a block being executed.
///
/// used to setup the state before execution of transactions.
#[derive(Debug, Clone)]
struct BlockData {
    misbehavior: Vec<tendermint::abci::types::Misbehavior>,
    height: tendermint::block::Height,
    time: tendermint::Time,
}

#[derive(Clone, Debug)]
struct PostTransactionExecutionResult {
    events: Vec<Event>,
    tx_results: Vec<ExecTxResult>,
    validator_updates: Vec<tendermint::validator::Update>,
}

#[derive(PartialEq)]
enum ExitContinue {
    Exit,
    Continue,
}

enum Proposal<'a> {
    Prepare(PrepareProposalInformation<'a>),
    Process(ProcessProposalInformation),
}

impl<'a> Proposal<'a> {
    fn current_tx_group(&self) -> Group {
        match self {
            Proposal::Prepare(vars) => vars.current_tx_group,
            Proposal::Process(vars) => vars.current_tx_group,
        }
    }

    fn set_current_tx_group(&mut self, group: Group) {
        match self {
            Proposal::Prepare(vars) => vars.current_tx_group = group,
            Proposal::Process(vars) => vars.current_tx_group = group,
        }
    }

    fn into_prepare(self) -> Option<PrepareProposalInformation<'a>> {
        match self {
            Proposal::Prepare(vars) => Some(vars),
            Proposal::Process(_) => None,
        }
    }

    fn into_process(self) -> Option<ProcessProposalInformation> {
        match self {
            Proposal::Prepare(_) => None,
            Proposal::Process(vars) => Some(vars),
        }
    }
}

struct PrepareProposalInformation<'a> {
    validated_txs: Vec<bytes::Bytes>,
    included_signed_txs: Vec<Transaction>,
    failed_tx_count: usize,
    execution_results: Vec<ExecTxResult>,
    excluded_txs: usize,
    current_tx_group: Group,
    mempool: Mempool,
    metrics: &'a Metrics,
}

impl<'a> PrepareProposalInformation<'a> {
    fn new(mempool: Mempool, metrics: &'a Metrics) -> Self {
        Self {
            validated_txs: Vec::new(),
            included_signed_txs: Vec::new(),
            failed_tx_count: 0,
            execution_results: Vec::new(),
            excluded_txs: 0,
            current_tx_group: Group::BundleableGeneral,
            mempool,
            metrics,
        }
    }
}

struct ProcessProposalInformation {
    execution_results: Vec<ExecTxResult>,
    current_tx_group: Group,
}

impl ProcessProposalInformation {
    fn new() -> Self {
        Self {
            execution_results: Vec::new(),
            current_tx_group: Group::BundleableGeneral,
        }
    }
}

#[instrument(skip_all)]
async fn proposal_checks_and_tx_execution(
    app: &mut App,
    tx: Arc<Transaction>,
    tx_hash_base_64: &str,
    block_size_constraints: &mut BlockSizeConstraints,
    proposal_args: &mut Proposal<'_>,
) -> Result<ExitContinue> {
    let tx_group = tx.group();
    let tx_sequence_data_bytes = tx_sequence_data_bytes(&tx);
    let bytes = tx.to_raw().encode_to_vec();
    let tx_len = bytes.len();

    let debug_msg = match proposal_args {
        Proposal::Prepare(_) => "excluding transaction",
        Proposal::Process(_) => "transaction error",
    };

    if let Proposal::Prepare(vars) = proposal_args {
        if !block_size_constraints.cometbft_has_space(tx_len) {
            vars.metrics
                .increment_prepare_proposal_excluded_transactions_cometbft_space();
            debug!(
                transaction_hash = %tx_hash_base_64,
                block_size_constraints = %json(&block_size_constraints),
                tx_data_bytes = tx_len,
                "excluding remaining transactions: max cometBFT data limit reached"
            );
            vars.excluded_txs = vars.excluded_txs.saturating_add(1);

            // break from loop, as the block is full
            return Ok(ExitContinue::Exit);
        }
    }

    if ExitContinue::Exit
        == check_sequencer_block_space(
            tx_hash_base_64,
            tx_sequence_data_bytes,
            block_size_constraints,
            debug_msg,
            proposal_args,
        )?
    {
        return Ok(ExitContinue::Exit);
    };
    check_tx_group(tx_hash_base_64, tx_group, debug_msg, proposal_args)?;
    handle_proposal_tx_execution(
        app,
        tx,
        tx_sequence_data_bytes,
        tx_len,
        tx_hash_base_64,
        block_size_constraints,
        proposal_args,
    )
    .await?;
    proposal_args.set_current_tx_group(tx_group);
    Ok(ExitContinue::Continue)
}

#[instrument(skip_all)]
fn check_sequencer_block_space(
    tx_hash_base64: &str,
    tx_sequence_data_bytes: usize,
    block_size_constraints: &BlockSizeConstraints,
    debug_msg: &str,
    proposal_vars: &mut Proposal,
) -> Result<ExitContinue> {
    if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
        debug!(
            transaction_hash = %tx_hash_base64,
            block_size_constraints = %json(&block_size_constraints),
            tx_data_bytes = tx_sequence_data_bytes,
            "{debug_msg}: max block sequenced data limit reached"
        );
        return match proposal_vars {
            Proposal::Prepare(vars) => {
                vars.metrics
                    .increment_prepare_proposal_excluded_transactions_sequencer_space();
                vars.excluded_txs = vars.excluded_txs.saturating_add(1);
                Ok(ExitContinue::Exit)
            }
            Proposal::Process(_) => Err(eyre!("max block sequenced data limit passed")),
        };
    }
    Ok(ExitContinue::Continue)
}

#[instrument(skip_all)]
fn check_tx_group(
    tx_hash_base64: &str,
    tx_group: Group,
    debug_msg: &str,
    proposal_vars: &mut Proposal,
) -> Result<()> {
    if tx_group > proposal_vars.current_tx_group() {
        debug!(
            transaction_hash = %tx_hash_base64,
            "{debug_msg}: group is higher priority than previously included transactions"
        );
        match proposal_vars {
            Proposal::Prepare(vars) => vars.excluded_txs = vars.excluded_txs.saturating_add(1),
            Proposal::Process(_) => {
                return Err(eyre!(
                    "transactions have incorrect transaction group ordering"
                ));
            }
        };
    }
    Ok(())
}

#[instrument(skip_all)]
async fn handle_proposal_tx_execution(
    app: &mut App,
    tx: Arc<Transaction>,
    tx_sequence_data_bytes: usize,
    tx_len: usize,
    tx_hash: &str,
    block_size_constraints: &mut BlockSizeConstraints,
    proposal_args: &mut Proposal<'_>,
) -> Result<()> {
    let (debug_msg, execution_results) = match proposal_args {
        Proposal::Prepare(vars) => ("excluding transaction", &mut vars.execution_results),
        Proposal::Process(vars) => ("transaction error", &mut vars.execution_results),
    };

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
            if let Proposal::Prepare(vars) = proposal_args {
                vars.validated_txs.push(tx.to_raw().encode_to_vec().into());
                vars.included_signed_txs.push((*tx).clone());
            }
        }
        Err(e) => {
            debug!(
                transaction_hash = %tx_hash,
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                "{debug_msg}: failed to execute transaction"
            );
            match proposal_args {
                Proposal::Prepare(vars) => {
                    vars.metrics
                        .increment_prepare_proposal_excluded_transactions_failed_execution();
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
                        vars.failed_tx_count = vars.failed_tx_count.saturating_add(1);

                        // remove the failing transaction from the mempool
                        //
                        // this will remove any transactions from the same sender
                        // as well, as the dependent nonces will not be able
                        // to execute
                        vars.mempool
                            .remove_tx_invalid(
                                tx,
                                RemovalReason::FailedPrepareProposal(e.to_string()),
                            )
                            .await;
                    }
                }
                Proposal::Process(_) => return Err(e.wrap_err("transaction failed to execute")),
            }
        }
    };

    Ok(())
}

fn tx_sequence_data_bytes(tx: &Arc<Transaction>) -> usize {
    tx.unsigned_transaction()
        .actions()
        .iter()
        .filter_map(Action::as_rollup_data_submission)
        .fold(0usize, |acc, seq| acc.saturating_add(seq.data.len()))
}
