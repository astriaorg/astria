#[cfg(test)]
pub(crate) mod test_utils;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_execute_transaction;

use std::{
    collections::VecDeque,
    sync::Arc,
};

use anyhow::{
    anyhow,
    ensure,
    Context,
};
use astria_core::{
    generated::protocol::transaction::v1alpha1 as raw,
    primitive::v1::Address,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::{
            Action,
            SignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::SequencerBlock,
};
use cnidarium::{
    ArcStateDeltaExt,
    Snapshot,
    StagedWriteBatch,
    StateDelta,
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
    block::Header,
    AppHash,
    Hash,
};
use tracing::{
    debug,
    info,
    instrument,
};

use crate::{
    accounts::{
        component::AccountsComponent,
        state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    api_state_ext::StateWriteExt as _,
    authority::{
        component::{
            AuthorityComponent,
            AuthorityComponentAppState,
        },
        state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    bridge::{
        component::BridgeComponent,
        state_ext::{
            StateReadExt as _,
            StateWriteExt,
        },
    },
    component::Component as _,
    genesis::GenesisState,
    ibc::component::IbcComponent,
    metrics_init,
    proposal::{
        block_size_constraints::BlockSizeConstraints,
        commitment::{
            generate_rollup_datas_commitment,
            GeneratedCommitments,
        },
    },
    sequence::component::SequenceComponent,
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::{
        self,
        InvalidNonce,
    },
};

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

    // The validator address in cometbft being used to sign votes.
    //
    // Used to avoid executing a block in both `prepare_proposal` and `process_proposal`. It
    // is set in `prepare_proposal` from information sent in from cometbft and can potentially
    // change round-to-round. In `process_proposal` we check if we prepared the proposal, and
    // if so, we clear the value and we skip re-execution of the block's transactions to avoid
    // failures caused by re-execution.
    validator_address: Option<account::Id>,

    // This is set to the executed hash of the proposal during `process_proposal`
    //
    // If it does not match the hash given during `begin_block`, then we clear and
    // reset the execution results cache + state delta. Transactions are re-executed.
    // If it does match, we utilize cached results to reduce computation.
    //
    // Resets to default hash at the beginning of `prepare_proposal`, and `process_proposal` if
    // `prepare_proposal` was not called.
    executed_proposal_hash: Hash,

    // cache of results of executing of transactions in `prepare_proposal` or `process_proposal`.
    // cleared at the end of each block.
    execution_results: Option<Vec<tendermint::abci::types::ExecTxResult>>,

    // the current `StagedWriteBatch` which contains the rocksdb write batch
    // of the current block being executed, created from the state delta,
    // and set after `finalize_block`.
    // this is committed to the state when `commit` is called, and set to `None`.
    write_batch: Option<StagedWriteBatch>,

    // the currently committed `AppHash` of the application state.
    // set whenever `commit` is called.
    //
    // allow clippy because we need be specific as to what hash this is.
    #[allow(clippy::struct_field_names)]
    app_hash: AppHash,
}

impl App {
    pub(crate) async fn new(snapshot: Snapshot) -> anyhow::Result<Self> {
        tracing::debug!("initializing App instance");

        let app_hash: AppHash = snapshot
            .root_hash()
            .await
            .context("failed to get current root hash")?
            .0
            .to_vec()
            .try_into()
            .expect("root hash conversion must succeed; should be 32 bytes");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Ok(Self {
            state,
            validator_address: None,
            executed_proposal_hash: Hash::default(),
            execution_results: None,
            write_batch: None,
            app_hash,
        })
    }

    #[instrument(name = "App:init_chain", skip_all)]
    pub(crate) async fn init_chain(
        &mut self,
        storage: Storage,
        genesis_state: GenesisState,
        genesis_validators: Vec<tendermint::validator::Update>,
        chain_id: String,
    ) -> anyhow::Result<AppHash> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        crate::asset::initialize_native_asset(&genesis_state.native_asset_base_denomination);
        state_tx.put_native_asset_denom(&genesis_state.native_asset_base_denomination);
        state_tx.put_chain_id_and_revision_number(chain_id.try_into().context("invalid chain ID")?);
        state_tx.put_block_height(0);

        for fee_asset in &genesis_state.allowed_fee_assets {
            state_tx.put_allowed_fee_asset(fee_asset.id());
        }

        // call init_chain on all components
        AccountsComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on AccountsComponent")?;
        AuthorityComponent::init_chain(
            &mut state_tx,
            &AuthorityComponentAppState {
                authority_sudo_address: genesis_state.authority_sudo_address,
                genesis_validators,
            },
        )
        .await
        .context("failed to call init_chain on AuthorityComponent")?;
        BridgeComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on BridgeComponent")?;
        IbcComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on IbcComponent")?;
        SequenceComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on SequenceComponent")?;

        state_tx.apply();

        let app_hash = self
            .prepare_commit(storage)
            .await
            .context("failed to prepare commit")?;
        debug!(app_hash = %telemetry::display::base64(&app_hash), "init_chain completed");
        Ok(app_hash)
    }

    fn update_state_for_new_round(&mut self, storage: &Storage) {
        // reset app state to latest committed state, in case of a round not being committed
        // but `self.state` was changed due to executing the previous round's data.
        //
        // if the previous round was committed, then the state stays the same.
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        // clear the cache of transaction execution results
        self.execution_results = None;
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
    ) -> anyhow::Result<abci::response::PrepareProposal> {
        self.validator_address = Some(prepare_proposal.proposer_address);
        self.update_state_for_new_round(&storage);

        let mut block_size_constraints = BlockSizeConstraints::new(
            usize::try_from(prepare_proposal.max_tx_bytes)
                .context("failed to convert max_tx_bytes to usize")?,
        )
        .context("failed to create block size constraints")?;

        let block_data = BlockData {
            misbehavior: prepare_proposal.misbehavior,
            height: prepare_proposal.height,
            time: prepare_proposal.time,
            next_validators_hash: prepare_proposal.next_validators_hash,
            proposer_address: prepare_proposal.proposer_address,
        };

        self.pre_execute_transactions(block_data)
            .await
            .context("failed to prepare for executing block")?;

        let (signed_txs, txs_to_include) = self
            .execute_transactions_before_finalization(
                prepare_proposal.txs,
                &mut block_size_constraints,
            )
            .await
            .context("failed to execute transactions")?;
        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(metrics_init::PROPOSAL_TRANSACTIONS).record(signed_txs.len() as f64);

        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in prepare_proposal")?;
        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(metrics_init::PROPOSAL_DEPOSITS).record(deposits.len() as f64);

        // generate commitment to sequence::Actions and deposits and commitment to the rollup IDs
        // included in the block
        let res = generate_rollup_datas_commitment(&signed_txs, deposits);

        Ok(abci::response::PrepareProposal {
            txs: res.into_transactions(txs_to_include),
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
    ) -> anyhow::Result<()> {
        // if we proposed this block (ie. prepare_proposal was called directly before this), then
        // we skip execution for this `process_proposal` call.
        //
        // if we didn't propose this block, `self.validator_address` will be None or a different
        // value, so we will execute the block as normal.
        if let Some(id) = self.validator_address {
            if id == process_proposal.proposer_address {
                debug!("skipping process_proposal as we are the proposer for this block");
                self.validator_address = None;
                self.executed_proposal_hash = process_proposal.hash;
                return Ok(());
            }
            metrics::counter!(metrics_init::PROCESS_PROPOSAL_SKIPPED_PROPOSAL).increment(1);
            debug!(
                "our validator address was set but we're not the proposer, so our previous \
                 proposal was skipped, executing block"
            );
            self.validator_address = None;
        }

        self.update_state_for_new_round(&storage);

        let mut txs = VecDeque::from(process_proposal.txs);
        let received_rollup_datas_root: [u8; 32] = txs
            .pop_front()
            .context("no transaction commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| anyhow!("transaction commitment must be 32 bytes"))?;

        let received_rollup_ids_root: [u8; 32] = txs
            .pop_front()
            .context("no chain IDs commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| anyhow!("chain IDs commitment must be 32 bytes"))?;

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
            .context("failed to prepare for executing block")?;

        // we don't care about the cometbft max_tx_bytes here, as cometbft would have
        // rejected the proposal if it was too large.
        // however, we should still validate the other constraints, namely
        // the max sequenced data bytes.
        let mut block_size_constraints = BlockSizeConstraints::new_unlimited_cometbft();

        let (signed_txs, txs_to_include) = self
            .execute_transactions_before_finalization(
                txs.into_iter().collect(),
                &mut block_size_constraints,
            )
            .await
            .context("failed to execute transactions")?;

        // all txs in the proposal should be deserializable and executable
        // if any txs were not deserializeable or executable, they would not have been
        // returned by `execute_block_data`, thus the length of `txs_to_include`
        // will be shorter than that of `txs`.
        ensure!(
            txs_to_include.len() == expected_txs_len,
            "transactions to be included do not match expected",
        );
        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(metrics_init::PROPOSAL_TRANSACTIONS).record(signed_txs.len() as f64);

        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in process_proposal")?;
        #[allow(clippy::cast_precision_loss)]
        metrics::histogram!(metrics_init::PROPOSAL_DEPOSITS).record(deposits.len() as f64);

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

        Ok(())
    }

    /// Executes and filters the given transaction data, writing it to the app's `StateDelta`.
    ///
    /// The result of execution of every transaction which is successfully decoded
    /// is stored in `self.execution_results`.
    ///
    /// Returns the transactions which were successfully decoded and executed
    /// in both their [`SignedTransaction`] and raw bytes form.
    ///
    /// Unlike the usual flow of an ABCI application, this is called during
    /// the proposal phase, ie. `prepare_proposal` or `process_proposal`.
    ///
    /// This is because we disallow transactions that fail execution to be included
    /// in a block's transaction data, as this would allow `sequence::Action`s to be
    /// included for free. Instead, we execute transactions during the proposal phase,
    /// and only include them in the block if they succeed.
    ///
    /// As a result, all transactions in a sequencer block are guaranteed to execute
    /// successfully.
    #[instrument(name = "App::execute_transactions_before_finalization", skip_all, fields(
        tx_count = txs.len()
    ))]
    async fn execute_transactions_before_finalization(
        &mut self,
        txs: Vec<bytes::Bytes>,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> anyhow::Result<(Vec<SignedTransaction>, Vec<bytes::Bytes>)> {
        let mut signed_txs: Vec<SignedTransaction> = Vec::with_capacity(txs.len());
        let mut validated_txs = Vec::with_capacity(txs.len());
        let mut excluded_tx_count: usize = 0;
        let mut execution_results = Vec::with_capacity(txs.len());

        for tx in txs {
            let tx_hash = Sha256::digest(&tx);

            // don't include tx if it would make the cometBFT block too large
            if !block_size_constraints.cometbft_has_space(tx.len()) {
                metrics::counter!(
                    metrics_init::PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE
                )
                .increment(1);
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    block_size_constraints = %json(&block_size_constraints),
                    tx_data_bytes = tx.len(),
                    "excluding transactions: max cometBFT data limit reached"
                );
                excluded_tx_count += 1;
                continue;
            }

            // try to decode the tx
            let signed_tx = match signed_transaction_from_bytes(&tx) {
                Err(e) => {
                    metrics::counter!(
                        metrics_init::PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE
                    )
                    .increment(1);
                    debug!(
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                        "failed to decode deliver tx payload to signed transaction; excluding it",
                    );
                    excluded_tx_count += 1;
                    continue;
                }
                Ok(tx) => tx,
            };

            // check if tx's sequence data will fit into sequence block
            let tx_sequence_data_bytes = signed_tx
                .unsigned_transaction()
                .actions
                .iter()
                .filter_map(Action::as_sequence)
                .fold(0usize, |acc, seq| acc + seq.data.len());

            if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
                metrics::counter!(
                    metrics_init::PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE
                )
                .increment(1);
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    block_size_constraints = %json(&block_size_constraints),
                    tx_data_bytes = tx_sequence_data_bytes,
                    "excluding transaction: max block sequenced data limit reached"
                );
                excluded_tx_count += 1;
                continue;
            }

            // execute tx and store in `execution_results` list
            match self.execute_transaction(signed_tx.clone()).await {
                Ok(events) => {
                    execution_results.push(ExecTxResult {
                        events,
                        ..Default::default()
                    });
                    block_size_constraints
                        .sequencer_checked_add(tx_sequence_data_bytes)
                        .context("error growing sequencer block size")?;
                    block_size_constraints
                        .cometbft_checked_add(tx.len())
                        .context("error growing cometBFT block size")?;
                    signed_txs.push(signed_tx);
                    validated_txs.push(tx);
                }
                Err(e) => {
                    metrics::counter!(
                        metrics_init::PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION
                    )
                    .increment(1);
                    debug!(
                        transaction_hash = %telemetry::display::base64(&tx_hash),
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                        "failed to execute transaction, not including in block"
                    );
                    excluded_tx_count += 1;
                }
            }
        }

        if excluded_tx_count > 0 {
            #[allow(clippy::cast_precision_loss)]
            metrics::gauge!(metrics_init::PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS)
                .set(excluded_tx_count as f64);
            info!(
                excluded_tx_count = excluded_tx_count,
                included_tx_count = validated_txs.len(),
                "excluded transactions from block"
            );
        }

        self.execution_results = Some(execution_results);
        Ok((signed_txs, validated_txs))
    }

    /// sets up the state for execution of the block's transactions.
    /// set the current height and timestamp, and calls `begin_block` on all components.
    ///
    /// this *must* be called anytime before a block's txs are executed, whether it's
    /// during the proposal phase, or finalize_block phase.
    #[instrument(name = "App::pre_execute_transactions", skip_all)]
    async fn pre_execute_transactions(&mut self, block_data: BlockData) -> anyhow::Result<()> {
        let chain_id = self
            .state
            .get_chain_id()
            .await
            .context("failed to get chain ID from state")?;

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
            .context("failed to call begin_block")?;

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
    ) -> anyhow::Result<abci::response::FinalizeBlock> {
        let chain_id = self
            .state
            .get_chain_id()
            .await
            .context("failed to get chain ID from state")?;

        // convert tendermint id to astria address; this assumes they are
        // the same address, as they are both ed25519 keys
        let proposer_address = finalize_block.proposer_address;
        let astria_proposer_address =
            Address::try_from_slice(finalize_block.proposer_address.as_bytes())
                .context("failed to convert proposer tendermint id to astria address")?;

        let height = finalize_block.height;
        let time = finalize_block.time;
        let Hash::Sha256(block_hash) = finalize_block.hash else {
            anyhow::bail!("finalized block hash is empty; this should not occur")
        };

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

        // cometbft expects a result for every tx in the block, so we need to return a
        // tx result for the commitments, even though they're not actually user txs.
        let mut tx_results: Vec<ExecTxResult> = Vec::with_capacity(finalize_block.txs.len());
        tx_results.extend(std::iter::repeat(ExecTxResult::default()).take(2));

        // When the hash is not empty, we have already executed and cached the results
        if self.executed_proposal_hash.is_empty() {
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
                .context("failed to execute block")?;

            // skip the first two transactions, as they are the rollup data commitments
            for tx in finalize_block.txs.iter().skip(2) {
                let signed_tx = signed_transaction_from_bytes(tx)
                    .context("protocol error; only valid txs should be finalized")?;

                match self.execute_transaction(signed_tx).await {
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
                            code: code.into(),
                            info: code.to_string(),
                            log: format!("{e:?}"),
                            ..Default::default()
                        });
                    }
                }
            }
        } else {
            let execution_results = self.execution_results.take().expect(
                "execution results must be present if txs were already executed during proposal \
                 phase",
            );
            tx_results.extend(execution_results);
        };

        let end_block = self
            .end_block(height.value(), astria_proposer_address)
            .await?;

        // get and clear block deposits from state
        let mut state_tx = StateDelta::new(self.state.clone());
        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in end_block")?;
        state_tx
            .clear_block_deposits()
            .await
            .context("failed to clear block deposits")?;
        debug!(
            deposits = %telemetry::display::json(&deposits),
            "got block deposits from state"
        );

        let sequencer_block = SequencerBlock::try_from_block_info_and_data(
            block_hash,
            chain_id,
            height,
            time,
            proposer_address,
            finalize_block
                .txs
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
            deposits,
        )
        .context("failed to convert block info and data to SequencerBlock")?;
        state_tx
            .put_sequencer_block(sequencer_block)
            .context("failed to write sequencer block to state")?;
        // events that occur after end_block are ignored here;
        // there should be none anyways.
        let _ = self.apply(state_tx);

        // prepare the `StagedWriteBatch` for a later commit.
        let app_hash = self
            .prepare_commit(storage.clone())
            .await
            .context("failed to prepare commit")?;

        Ok(abci::response::FinalizeBlock {
            events: end_block.events,
            validator_updates: end_block.validator_updates,
            consensus_param_updates: end_block.consensus_param_updates,
            tx_results,
            app_hash,
        })
    }

    async fn prepare_commit(&mut self, storage: Storage) -> anyhow::Result<AppHash> {
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
        state.put_storage_version_by_height(height, new_version);
        debug!(
            height,
            version = new_version,
            "stored storage version for height"
        );

        let write_batch = storage
            .prepare_commit(state)
            .await
            .context("failed to prepare commit")?;
        let app_hash: AppHash = write_batch
            .root_hash()
            .0
            .to_vec()
            .try_into()
            .context("failed to convert app hash")?;
        self.write_batch = Some(write_batch);
        Ok(app_hash)
    }

    #[instrument(name = "App::begin_block", skip_all)]
    pub(crate) async fn begin_block(
        &mut self,
        begin_block: &abci::request::BeginBlock,
    ) -> anyhow::Result<Vec<abci::Event>> {
        let mut state_tx = StateDelta::new(self.state.clone());

        // store the block height
        state_tx.put_block_height(begin_block.header.height.into());
        // store the block time
        state_tx.put_block_timestamp(begin_block.header.time);

        // call begin_block on all components
        let mut arc_state_tx = Arc::new(state_tx);
        AccountsComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on AccountsComponent")?;
        AuthorityComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on AuthorityComponent")?;
        BridgeComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on BridgeComponent")?;
        IbcComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on IbcComponent")?;
        SequenceComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on SequenceComponent")?;

        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        Ok(self.apply(state_tx))
    }

    /// Executes a signed transaction.
    #[instrument(name = "App::execute_transaction", skip_all, fields(
        signed_transaction_hash = %telemetry::display::base64(&signed_tx.sha256_of_proto_encoding()),
        sender = %Address::from_verification_key(signed_tx.verification_key()),
    ))]
    pub(crate) async fn execute_transaction(
        &mut self,
        signed_tx: astria_core::protocol::transaction::v1alpha1::SignedTransaction,
    ) -> anyhow::Result<Vec<abci::Event>> {
        let signed_tx_2 = signed_tx.clone();
        let stateless =
            tokio::spawn(async move { transaction::check_stateless(&signed_tx_2).await });
        let signed_tx_2 = signed_tx.clone();
        let state2 = self.state.clone();
        let stateful =
            tokio::spawn(async move { transaction::check_stateful(&signed_tx_2, &state2).await });

        stateless
            .await
            .context("stateless check task aborted while executing")?
            .context("stateless check failed")?;
        stateful
            .await
            .context("stateful check task aborted while executing")?
            .context("stateful check failed")?;
        // At this point, the stateful checks should have completed,
        // leaving us with exclusive access to the Arc<State>.
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should be present and unique");

        transaction::execute(&signed_tx, &mut state_tx)
            .await
            .context("failed executing transaction")?;
        let (_, events) = state_tx.apply();

        info!(event_count = events.len(), "executed transaction");
        Ok(events)
    }

    #[instrument(name = "App::end_block", skip_all)]
    pub(crate) async fn end_block(
        &mut self,
        height: u64,
        proposer_address: Address,
    ) -> anyhow::Result<abci::response::EndBlock> {
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
            .context("failed to call end_block on AccountsComponent")?;
        AuthorityComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .context("failed to call end_block on AuthorityComponent")?;
        BridgeComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .context("failed to call end_block on BridgeComponent")?;
        IbcComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .context("failed to call end_block on IbcComponent")?;
        SequenceComponent::end_block(&mut arc_state_tx, &end_block)
            .await
            .context("failed to call end_block on SequenceComponent")?;

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
        let fees = self
            .state
            .get_block_fees()
            .await
            .context("failed to get block fees")?;

        for (asset, amount) in fees {
            let balance = state_tx
                .get_account_balance(proposer_address, asset)
                .await
                .context("failed to get proposer account balance")?;
            let new_balance = balance
                .checked_add(amount)
                .context("account balance overflowed u128")?;
            state_tx
                .put_account_balance(proposer_address, asset, new_balance)
                .context("failed to put proposer account balance")?;
        }

        // clear block fees
        state_tx.clear_block_fees().await;

        let events = self.apply(state_tx);
        Ok(abci::response::EndBlock {
            validator_updates: validator_updates.into_tendermint_validator_updates(),
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
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));
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

/// relevant data of a block being executed.
///
/// used to setup the state before execution of transactions.
#[derive(Debug, Clone)]
pub(crate) struct BlockData {
    misbehavior: Vec<tendermint::abci::types::Misbehavior>,
    height: tendermint::block::Height,
    time: tendermint::Time,
    next_validators_hash: Hash,
    proposer_address: account::Id,
}

fn signed_transaction_from_bytes(bytes: &[u8]) -> anyhow::Result<SignedTransaction> {
    let raw = raw::SignedTransaction::decode(bytes)
        .context("failed to decode protobuf to signed transaction")?;
    let tx = SignedTransaction::try_from_raw(raw)
        .context("failed to transform raw signed transaction to verified type")?;

    Ok(tx)
}
