use std::{
    collections::VecDeque,
    sync::Arc,
};

use astria_core::{
    protocol::{
        abci::AbciErrorCode,
        genesis::v1::GenesisAppState,
        transaction::v1::{
            action::ValidatorUpdate,
            Transaction,
        },
    },
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
    ArcStateDeltaExt,
    StateDelta,
    StateRead,
    Storage,
};
use prost::Message as _;
use tendermint::{
    abci::{
        self,
        types::ExecTxResult,
        Code,
    },
    AppHash,
};
use tracing::{
    debug,
    instrument,
};

use super::{
    App,
    BlockData,
    Mempool,
};
use crate::{
    accounts::component::AccountsComponent,
    address::StateWriteExt as _,
    app::{
        PostTransactionExecutionResult,
        StateWriteExt as _,
        EXECUTION_RESULTS_KEY,
        POST_TRANSACTION_EXECUTION_RESULT_KEY,
    },
    assets::StateWriteExt as _,
    authority::genesis::{
        AuthorityComponent,
        AuthorityComponentAppState,
    },
    bridge::StateReadExt as _,
    fees::genesis::FeesComponent,
    genesis::Genesis as _,
    ibc::genesis::IbcComponent,
    proposal::{
        block_size_constraints::BlockSizeConstraints,
        commitment::{
            generate_rollup_datas_commitment,
            GeneratedCommitments,
        },
    },
    transaction::InvalidNonce,
};

#[async_trait::async_trait]
pub(crate) trait AppAbci {
    async fn commit(&mut self, storage: Storage);
    async fn finalize_block(
        &mut self,
        finalize_block: abci::request::FinalizeBlock,
        storage: Storage,
    ) -> Result<abci::response::FinalizeBlock>;
    async fn init_chain(
        &mut self,
        snapshot: Storage,
        genesis_state: GenesisAppState,
        genesis_validators: Vec<ValidatorUpdate>,
        chain_id: String,
    ) -> Result<AppHash>;
    async fn prepare_proposal(
        &mut self,
        prepare_proposal: abci::request::PrepareProposal,
        storage: Storage,
    ) -> Result<abci::response::PrepareProposal>;
    async fn process_proposal(
        &mut self,
        process_proposal: abci::request::ProcessProposal,
        storage: Storage,
    ) -> Result<()>;
}

#[async_trait::async_trait]
impl AppAbci for App {
    #[instrument(name = "App::commit", skip_all)]
    async fn commit(&mut self, storage: Storage) {
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

    /// Executes the given block, but does not write it to disk.
    ///
    /// `commit` must be called after this to write the block to disk.
    ///
    /// This is called by cometbft after the block has already been
    /// committed by the network's consensus.
    #[instrument(name = "App::finalize_block", skip_all)]
    async fn finalize_block(
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
            };

            self.prepare_state_for_execution(block_data)
                .await
                .wrap_err("failed to prepare app state for execution")?;

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

            self.update_state_and_end_block(
                finalize_block.hash,
                height,
                time,
                proposer_address,
                finalize_block.txs,
                tx_results,
            )
            .await
            .wrap_err("failed to update state post-execution")?;
        }

        // update the priority of any txs in the mempool based on the updated app state
        if self.recost_mempool {
            self.metrics.increment_mempool_recosted();
        }
        update_mempool_after_finalization(&mut self.mempool, &self.state, self.recost_mempool)
            .await;

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
        let finalize_block = abci::response::FinalizeBlock {
            events: post_transaction_execution_result.events,
            validator_updates: post_transaction_execution_result.validator_updates,
            consensus_param_updates: None,
            app_hash,
            tx_results: post_transaction_execution_result.tx_results,
        };

        Ok(finalize_block)
    }

    #[instrument(name = "App:init_chain", skip_all)]
    async fn init_chain(
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

    /// Generates a commitment to the `sequence::Actions` in the block's transactions.
    ///
    /// This is required so that a rollup can easily verify that the transactions it
    /// receives are correct (ie. we actually included in a sequencer block, and none
    /// are missing)
    /// It puts this special "commitment" as the first transaction in a block.
    /// When other validators receive the block, they know the first transaction is
    /// supposed to be the commitment, and verifies that is it correct.
    #[instrument(name = "App::prepare_proposal", skip_all)]
    async fn prepare_proposal(
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
        };

        self.prepare_state_for_execution(block_data)
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
        Ok(abci::response::PrepareProposal {
            txs,
        })
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions
    /// and ensures it matches the commitment created by the proposer, which
    /// should be the first transaction in the block.
    #[instrument(name = "App::process_proposal", skip_all)]
    async fn process_proposal(
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
                let Some(tx_results) = self.state.object_get(EXECUTION_RESULTS_KEY) else {
                    bail!("execution results must be present after executing transactions")
                };

                self.update_state_and_end_block(
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
        };

        self.prepare_state_for_execution(block_data)
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

        self.executed_proposal_hash = process_proposal.hash;
        self.update_state_and_end_block(
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

fn signed_transaction_from_bytes(bytes: &[u8]) -> Result<Transaction> {
    use astria_core::generated::protocol::transaction::v1 as Raw;

    let raw = Raw::Transaction::decode(bytes)
        .wrap_err("failed to decode protobuf to signed transaction")?;
    let tx = Transaction::try_from_raw(raw)
        .wrap_err("failed to transform raw signed transaction to verified type")?;

    Ok(tx)
}
