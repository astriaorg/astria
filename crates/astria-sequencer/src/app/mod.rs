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

pub(crate) mod vote_extension;

use std::sync::Arc;

use astria_core::{
    primitive::v1::TRANSACTION_ID_LEN,
    protocol::{
        abci::AbciErrorCode,
        connect::v1::ExtendedCommitInfoWithCurrencyPairMapping,
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
    sequencerblock::v1::{
        block::{
            self,
            ParsedDataItems,
            SequencerBlockBuilder,
        },
        DataItem,
    },
    upgrades::v1::{
        ChangeHash,
        Upgrade,
        Upgrades,
    },
    Protobuf as _,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        ensure,
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
use prost::Message as _;
use sha2::{
    Digest as _,
    Sha256,
};
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
    consensus::params::VersionParams,
    AppHash,
    Hash,
    Time,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
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
    action_handler::{
        impls::transaction::InvalidNonce,
        ActionHandler as _,
    },
    address::StateWriteExt as _,
    app::vote_extension::ProposalHandler,
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
    connect::{
        market_map::component::MarketMapComponent,
        oracle::component::OracleComponent,
    },
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
    upgrades::{
        get_consensus_params_from_cometbft,
        should_shut_down,
        StateWriteExt as _,
    },
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

    upgrades: Upgrades,

    cometbft_rpc_addr: String,

    // used to create and verify vote extensions, if this is a validator node.
    vote_extension_handler: vote_extension::Handler,

    metrics: &'static Metrics,
}

impl App {
    #[instrument(name = "App::new", skip_all, err)]
    pub(crate) async fn new(
        snapshot: Snapshot,
        mempool: Mempool,
        upgrades: Upgrades,
        cometbft_rpc_addr: String,
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

        Ok(Self {
            state,
            mempool,
            executed_proposal_fingerprint: None,
            executed_proposal_hash: Hash::default(),
            recost_mempool: false,
            write_batch: None,
            app_hash,
            upgrades,
            cometbft_rpc_addr,
            vote_extension_handler,
            metrics,
        })
    }

    #[instrument(name = "App:init_chain", skip_all, err)]
    pub(crate) async fn init_chain(
        &mut self,
        storage: Storage,
        genesis_state: GenesisAppState,
        genesis_validators: Vec<ValidatorUpdate>,
        chain_id: String,
        consensus_params: tendermint::consensus::Params,
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
        state_tx
            .put_consensus_params(consensus_params.clone())
            .wrap_err("failed to write consensus params to state")?;

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

        if vote_extensions_enable_height(&consensus_params) != VOTE_EXTENSIONS_DISABLED_HEIGHT {
            MarketMapComponent::init_chain(&mut state_tx, &genesis_state)
                .await
                .wrap_err("init_chain failed on MarketMapComponent")?;
            OracleComponent::init_chain(&mut state_tx, &genesis_state)
                .await
                .wrap_err("init_chain failed on OracleComponent")?;
        }

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
    #[instrument(name = "App::prepare_proposal", skip_all, err(level = Level::WARN))]
    pub(crate) async fn prepare_proposal(
        &mut self,
        prepare_proposal: abci::request::PrepareProposal,
        storage: Storage,
    ) -> Result<abci::response::PrepareProposal> {
        self.executed_proposal_fingerprint = Some(prepare_proposal.clone().into());
        self.update_state_for_new_round(&storage);

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
        let encoded_upgrade_change_hashes = upgrade_change_hashes
            .map(|hashes| DataItem::UpgradeChangeHashes(hashes).encode())
            .transpose()
            .wrap_err("failed to encode upgrade change hashes")?;

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
            .encode()
            .wrap_err("failed to encode extended commit info")?;

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
                encoded_extended_commit_info = DataItem::ExtendedCommitInfo(Bytes::new())
                    .encode()
                    .wrap_err("failed to encode empty extended commit info")?;
                block_size_constraints
                    .cometbft_checked_add(encoded_extended_commit_info.len())
                    .wrap_err("exceeded size limit while adding empty extended commit info")?;
            }

            Some(encoded_extended_commit_info)
        } else {
            None
        };

        // ignore the txs passed by cometbft in favour of our app-side mempool
        let (included_tx_bytes, signed_txs_included) = self
            .prepare_proposal_tx_execution(block_size_constraints)
            .await
            .wrap_err("failed to execute transactions")?;
        self.metrics
            .record_proposal_transactions(signed_txs_included.len());

        let deposits = self.state.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        // generate commitment to sequence::Actions and deposits and commitment to the rollup IDs
        // included in the block, chain on the extended commit info if `Some`, and finally chain on
        // the tx bytes.
        let commitments_iter = if uses_data_item_enum {
            generate_rollup_datas_commitment::<true>(&signed_txs_included, deposits).into_iter()
        } else {
            generate_rollup_datas_commitment::<false>(&signed_txs_included, deposits).into_iter()
        }
        .wrap_err("failed to generate commitments")?;

        let txs = commitments_iter
            .chain(encoded_upgrade_change_hashes.into_iter())
            .chain(encoded_extended_commit_info.into_iter())
            .chain(included_tx_bytes)
            .collect();

        Ok(abci::response::PrepareProposal {
            txs,
        })
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
        let uses_data_item_enum = self.uses_data_item_enum(process_proposal.height);
        let parsed_data_items = if uses_data_item_enum {
            let with_extended_commit_info = self
                .vote_extensions_enabled(process_proposal.height)
                .await?;
            ParsedDataItems::new_from_typed_data(&process_proposal.txs, with_extended_commit_info)
        } else {
            ParsedDataItems::new_from_untyped_data(&process_proposal.txs)
        }
        .wrap_err("failed to parse data items")?;

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
                let Some(tx_results) = self.state.object_get(EXECUTION_RESULTS_KEY) else {
                    bail!("execution results must be present after executing transactions")
                };

                self.post_execute_transactions(
                    process_proposal.hash,
                    process_proposal.height,
                    process_proposal.time,
                    process_proposal.proposer_address,
                    parsed_data_items,
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

        if let Some(extended_commit_info_with_proof) =
            &parsed_data_items.extended_commit_info_with_proof
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
            &parsed_data_items,
            upgrade_change_hashes.as_ref(),
        )?;

        // we don't care about the cometbft max_tx_bytes here, as cometbft would have
        // rejected the proposal if it was too large.
        // however, we should still validate the other constraints, namely
        // the max sequenced data bytes.
        let block_size_constraints = BlockSizeConstraints::new_unlimited_cometbft();

        let tx_results = self
            .process_proposal_tx_execution(
                &parsed_data_items.rollup_transactions,
                block_size_constraints,
            )
            .await
            .wrap_err("failed to execute transactions")?;

        self.metrics
            .record_proposal_transactions(parsed_data_items.rollup_transactions.len());

        let deposits = self.state.get_cached_block_deposits();
        self.metrics.record_proposal_deposits(deposits.len());

        let (expected_rollup_datas_root, expected_rollup_ids_root) = if uses_data_item_enum {
            let commitments = generate_rollup_datas_commitment::<true>(
                &parsed_data_items.rollup_transactions,
                deposits,
            );
            (commitments.rollup_datas_root, commitments.rollup_ids_root)
        } else {
            let commitments = generate_rollup_datas_commitment::<false>(
                &parsed_data_items.rollup_transactions,
                deposits,
            );
            (commitments.rollup_datas_root, commitments.rollup_ids_root)
        };
        ensure!(
            parsed_data_items.rollup_transactions_root == expected_rollup_datas_root,
            "rollup transactions commitment does not match expected",
        );
        ensure!(
            parsed_data_items.rollup_ids_root == expected_rollup_ids_root,
            "rollup IDs commitment does not match expected",
        );

        self.executed_proposal_hash = process_proposal.hash;
        self.post_execute_transactions(
            process_proposal.hash,
            process_proposal.height,
            process_proposal.time,
            process_proposal.proposer_address,
            parsed_data_items,
            tx_results,
        )
        .await
        .wrap_err("failed to run post execute transactions handler")
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
    ) -> Result<(Vec<Bytes>, Vec<Arc<Transaction>>)> {
        let mempool_len = self.mempool.len().await;
        debug!(mempool_len, "executing transactions from mempool");

        let mut proposal_info = Proposal::Prepare {
            block_size_constraints,
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

            if self
                .proposal_checks_and_tx_execution(tx, Some(tx_hash), &mut proposal_info)
                .await?
                .should_break()
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
        txs: &[Arc<Transaction>],
        block_size_constraints: BlockSizeConstraints,
    ) -> Result<Vec<ExecTxResult>> {
        let mut proposal_info = Proposal::Process {
            block_size_constraints,
            execution_results: vec![],
            current_tx_group: Group::BundleableGeneral,
        };

        for tx in txs {
            if self
                .proposal_checks_and_tx_execution(tx.clone(), None, &mut proposal_info)
                .await?
                .should_break()
            {
                break;
            }
        }
        Ok(proposal_info.execution_results())
    }

    #[instrument(skip_all)]
    async fn proposal_checks_and_tx_execution(
        &mut self,
        tx: Arc<Transaction>,
        // `prepare_proposal_tx_execution` already has the tx hash, so we pass it in here
        tx_hash: Option<[u8; TRANSACTION_ID_LEN]>,
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
            block_size_constraints,
            metrics,
            excluded_txs,
            ..
        } = proposal_info
        {
            if !block_size_constraints.cometbft_has_space(tx_len) {
                metrics.increment_prepare_proposal_excluded_transactions_cometbft_space();
                debug!(
                    transaction_hash = %tx_hash_base_64,
                    block_size_constraints = %json(block_size_constraints),
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
        if !proposal_info
            .block_size_constraints()
            .sequencer_has_space(tx_sequence_data_bytes)
        {
            debug!(
                transaction_hash = %tx_hash_base_64,
                block_size_constraints = %json(&proposal_info.block_size_constraints()),
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
        match self.execute_transaction(tx.clone()).await {
            Ok(events) => {
                execution_results.push(ExecTxResult {
                    events,
                    ..Default::default()
                });
                proposal_info
                    .block_size_constraints_mut()
                    .sequencer_checked_add(tx_sequence_data_bytes)
                    .wrap_err("error growing sequencer block size")?;
                proposal_info
                    .block_size_constraints_mut()
                    .cometbft_checked_add(tx_len)
                    .wrap_err("error growing cometBFT block size")?;
                if let Proposal::Prepare {
                    validated_txs,
                    included_signed_txs,
                    ..
                } = proposal_info
                {
                    validated_txs.push(tx_bytes.into());
                    included_signed_txs.push(tx.clone());
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
    async fn pre_execute_transactions(
        &mut self,
        block_data: BlockData,
    ) -> Result<Option<Vec<ChangeHash>>> {
        let upgrade_change_hashes = self
            .execute_upgrade_if_due(block_data.height)
            .wrap_err("failed to execute upgrade")?;
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
        self.vote_extension_handler.extend_vote(&self.state).await
    }

    pub(crate) async fn verify_vote_extension(
        &mut self,
        vote_extension: abci::request::VerifyVoteExtension,
    ) -> Result<abci::response::VerifyVoteExtension> {
        self.vote_extension_handler
            .verify_vote_extension(&self.state, vote_extension)
            .await
    }

    /// updates the app state after transaction execution, and generates the resulting
    /// `SequencerBlock`.
    ///
    /// this must be called after a block's transactions are executed.
    #[instrument(name = "App::post_execute_transactions", skip_all, err(level = Level::WARN))]
    async fn post_execute_transactions(
        &mut self,
        block_hash: Hash,
        height: tendermint::block::Height,
        time: tendermint::Time,
        proposer_address: account::Id,
        parsed_data_items: ParsedDataItems,
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
        // the tx_results passed to this function only contain results for every user
        // transaction, not the injected ones.
        let non_rollup_tx_count = parsed_data_items.non_rollup_transaction_count();
        let mut finalize_block_tx_results: Vec<ExecTxResult> =
            Vec::with_capacity(parsed_data_items.rollup_transactions.len());
        finalize_block_tx_results
            .extend(std::iter::repeat(ExecTxResult::default()).take(non_rollup_tx_count));
        finalize_block_tx_results.extend(tx_results);

        let sequencer_block = SequencerBlockBuilder {
            block_hash: block::Hash::new(block_hash),
            chain_id,
            height,
            time,
            proposer_address,
            parsed_data_items,
            deposits: deposits_in_this_block,
        }
        .try_build()
        .wrap_err("failed to convert block info and data to SequencerBlock")?;
        state_tx
            .put_sequencer_block(sequencer_block)
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

        Ok(())
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
        // If we previously executed txs in a different proposal than is being processed,
        // reset cached state changes.
        if self.executed_proposal_hash != finalize_block.hash {
            self.update_state_for_new_round(&storage);
        }

        let uses_data_item_enum = self.uses_data_item_enum(finalize_block.height);
        let parsed_data_items = if uses_data_item_enum {
            let with_extended_commit_info =
                self.vote_extensions_enabled(finalize_block.height).await?;
            ParsedDataItems::new_from_typed_data(&finalize_block.txs, with_extended_commit_info)
        } else {
            ParsedDataItems::new_from_untyped_data(&finalize_block.txs)
        }
        .wrap_err("failed to parse data items")?;

        if let Some(extended_commit_info_with_proof) =
            &parsed_data_items.extended_commit_info_with_proof
        {
            let extended_commit_info = extended_commit_info_with_proof.extended_commit_info();
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
            let _ = self.apply(state_tx);
        }

        // When the hash is not empty, we have already executed and cached the results
        if self.executed_proposal_hash.is_empty() {
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
                &parsed_data_items,
                upgrade_change_hashes.as_ref(),
            )?;

            let mut tx_results = Vec::with_capacity(parsed_data_items.rollup_transactions.len());
            for tx in &parsed_data_items.rollup_transactions {
                match self.execute_transaction(tx.clone()).await {
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
                finalize_block.time,
                finalize_block.proposer_address,
                parsed_data_items,
                tx_results,
            )
            .await
            .wrap_err("failed to run post execute transactions handler")?;
        }

        // update the priority of any txs in the mempool based on the updated app state
        if self.recost_mempool {
            self.metrics.increment_mempool_recosted();
        }
        update_mempool_after_finalization(&mut self.mempool, &self.state, self.recost_mempool)
            .await;

        let PostTransactionExecutionResult {
            events,
            tx_results,
            validator_updates,
            mut consensus_param_updates,
        } = self
            .state
            .object_get(POST_TRANSACTION_EXECUTION_RESULT_KEY)
            .expect(
                "post_transaction_execution_result must be present, as txs were already executed \
                 just now or during the proposal phase",
            );

        self.update_consensus_params_if_upgrade_due(
            finalize_block.height,
            &mut consensus_param_updates,
        )
        .await
        .wrap_err("failed to update consensus params")?;
        if let Some(consensus_params) = consensus_param_updates.clone() {
            info!(
                consensus_params = %display_consensus_params(&consensus_params),
                "updated consensus params"
            );
            let mut delta_delta = StateDelta::new(self.state.clone());
            delta_delta
                .put_consensus_params(consensus_params)
                .wrap_err("failed to put consensus params to storage")?;
            let _ = self.apply(delta_delta);
        }

        // prepare the `StagedWriteBatch` for a later commit.
        let app_hash = self
            .prepare_commit(storage)
            .await
            .wrap_err("failed to prepare commit")?;
        let finalize_block = abci::response::FinalizeBlock {
            events,
            tx_results,
            validator_updates,
            consensus_param_updates,
            app_hash,
        };

        Ok(finalize_block)
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
        MarketMapComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on MarketMapComponent")?;
        OracleComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .wrap_err("begin_block failed on OracleComponent")?;

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
                .for_each(|attr| attr.index = true);
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
        MarketMapComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on MarketMapComponent")?;
        OracleComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .wrap_err("end_block failed on OracleComponent")?;

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
    pub(crate) async fn commit(&mut self, storage: Storage) -> Result<ShouldShutDown> {
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
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        should_shut_down(&self.upgrades, &storage.latest_snapshot()).await
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

    /// Execute any changes to global state required as part of any upgrade with an activation
    /// height == `block_height`.
    ///
    /// At a minimum, the `info` of each `Change` in such an upgrade must be written to verifiable
    /// storage.
    ///
    /// Returns `Ok(None)` if no upgrade was executed, or `Ok(Some(hashes of executed changes))` if
    /// an upgrade was executed.
    fn execute_upgrade_if_due(
        &mut self,
        block_height: tendermint::block::Height,
    ) -> Result<Option<Vec<ChangeHash>>> {
        let Some(upgrade) = self
            .upgrades
            .upgrade_activating_at_height(block_height.value())
        else {
            return Ok(None);
        };
        let mut delta_delta = StateDelta::new(self.state.clone());
        let upgrade_name = upgrade.name();
        let mut change_hashes = vec![];
        for change in upgrade.changes() {
            change_hashes.push(change.calculate_hash());
            delta_delta
                .put_upgrade_change_info(&upgrade_name, change)
                .wrap_err("failed to put upgrade change info")?;
            info!(upgrade = %upgrade_name, change = %change.name(), "executed upgrade change");
        }

        // NOTE: any further state changes specific to individual upgrades should be
        //       executed here after matching on the upgrade variant.

        #[expect(
            irrefutable_let_patterns,
            reason = "will become refutable once we have more than one upgrade variant"
        )]
        if let Upgrade::Upgrade1(upgrade_1) = upgrade {
            let genesis_state = upgrade_1.connect_oracle_change().genesis();
            MarketMapComponent::handle_genesis(&mut delta_delta, genesis_state.market_map())
                .wrap_err("failed to handle market map genesis")?;
            info!("handled market map genesis");
            OracleComponent::handle_genesis(&mut delta_delta, genesis_state.oracle())
                .wrap_err("failed to handle oracle genesis")?;
            info!("handled oracle genesis");
        }

        let _ = self.apply(delta_delta);

        Ok(Some(change_hashes))
    }

    /// Updates `params` with any changes to CometBFT consensus params required as part of any
    /// upgrade with an activation height == `block_height`.
    ///
    /// If no upgrade is due, this is a no-op. Otherwise, `params` is updated if `Some` or set to
    /// `Some` if passed as `None`.
    ///
    /// At a minimum, the ABCI application version should be increased.
    ///
    /// NOTE: the updated params are NOT put to storage - this needs to be done after calling this
    ///       method if the params are `Some`.
    async fn update_consensus_params_if_upgrade_due(
        &mut self,
        block_height: tendermint::block::Height,
        maybe_params: &mut Option<tendermint::consensus::Params>,
    ) -> Result<()> {
        let Some(upgrade) = self
            .upgrades
            .upgrade_activating_at_height(block_height.value())
        else {
            return Ok(());
        };

        let mut params = match maybe_params.take() {
            Some(value) => value,
            None => {
                match self
                    .state
                    .get_consensus_params()
                    .await
                    .wrap_err("failed to get consensus params from storage")?
                {
                    Some(value) => value,
                    None => get_consensus_params_from_cometbft(
                        &self.cometbft_rpc_addr,
                        block_height.value(),
                    )
                    .await
                    .wrap_err("failed to get consensus params from cometbft")?,
                }
            }
        };

        let new_app_version = upgrade.app_version();
        if let Some(existing_app_version) = &params.version {
            if new_app_version <= existing_app_version.app {
                error!(
                    "new app version {new_app_version} should be greater than existing version {}",
                    existing_app_version.app
                );
            }
        }
        params.version = Some(VersionParams {
            app: new_app_version,
        });

        // NOTE: any further changes specific to individual upgrades should be applied here after
        //       matching on the upgrade variant.

        #[expect(
            irrefutable_let_patterns,
            reason = "will become refutable once we have more than one upgrade variant"
        )]
        if let Upgrade::Upgrade1(_) = upgrade {
            set_vote_extensions_enable_height_to_next_block_height(block_height, &mut params);
        }

        *maybe_params = Some(params);
        Ok(())
    }

    /// Returns whether or not the block at the given height uses encoded `DataItem`s as the raw
    /// `txs` field of `response::PrepareProposal`, and hence also `request::ProcessProposal` and
    /// `request::FinalizeBlock`.
    ///
    /// This behavior was introduced in `Upgrade1`.  If `Upgrade1` is not included in the upgrades
    /// files, the assumption is that this network uses `DataItem`s from genesis onwards.
    ///
    /// Returns `true` if and only if:
    /// - `Upgrade1` is in `self.upgrades` and `block_height` is greater than or equal to its
    ///   activation height, or
    /// - `Upgrade1` is not in `self.upgrades`.
    fn uses_data_item_enum(&self, block_height: tendermint::block::Height) -> bool {
        self.upgrades.upgrade_1().map_or(true, |upgrade_1| {
            block_height.value() >= upgrade_1.activation_height()
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
}

fn vote_extensions_enable_height(consensus_params: &tendermint::consensus::Params) -> u64 {
    consensus_params
        .abci
        .vote_extensions_enable_height
        .map_or(0, |height| height.value())
}

fn set_vote_extensions_enable_height_to_next_block_height(
    current_block_height: tendermint::block::Height,
    consensus_params: &mut tendermint::consensus::Params,
) {
    // Set the vote_extensions_enable_height as the next block height (it must be a future
    // height to be valid).
    let new_enable_height = current_block_height.increment();
    if let Some(existing_enable_height) = consensus_params.abci.vote_extensions_enable_height {
        // If vote extensions are already enabled, they cannot be disabled, and the
        // `vote_extensions_enable_height` cannot be changed.
        if existing_enable_height.value() != VOTE_EXTENSIONS_DISABLED_HEIGHT {
            error!(
                %existing_enable_height, %new_enable_height,
                "vote extensions enable height already set; ignoring update",
            );
            return;
        }
    }
    consensus_params.abci.vote_extensions_enable_height = Some(new_enable_height);
}

fn ensure_upgrade_change_hashes_as_expected(
    received_data_items: &ParsedDataItems,
    calculated_upgrade_change_hashes: Option<&Vec<ChangeHash>>,
) -> Result<()> {
    match (
        &received_data_items.upgrade_change_hashes_with_proof,
        calculated_upgrade_change_hashes,
    ) {
        (Some(received_hashes), Some(calculated_hashes)) => {
            ensure!(
                received_hashes.upgrade_change_hashes() == calculated_hashes,
                "upgrade change hashes ({:?}) do not match expected ({calculated_hashes:?})",
                received_hashes.upgrade_change_hashes()
            );
            Ok(())
        }
        (None, None) => Ok(()),
        (Some(_), None) => bail!("received upgrade change hashes, but no upgrade due"),
        (None, Some(_)) => bail!("upgrade due, but didn't receive upgrade change hashes"),
    }
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
        validated_txs: Vec<Bytes>,
        included_signed_txs: Vec<Arc<Transaction>>,
        failed_tx_count: usize,
        execution_results: Vec<ExecTxResult>,
        excluded_txs: usize,
        current_tx_group: Group,
        mempool: Mempool,
        metrics: &'static Metrics,
    },
    Process {
        block_size_constraints: BlockSizeConstraints,
        execution_results: Vec<ExecTxResult>,
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
