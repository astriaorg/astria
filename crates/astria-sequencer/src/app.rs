use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    sync::Arc,
};

use anyhow::{
    anyhow,
    ensure,
    Context,
};
use astria_core::{
    generated::sequencer::v1 as raw,
    sequencer::v1::{
        block::Deposit,
        transaction::Action,
        Address,
        RollupId,
        SequencerBlock,
        SignedTransaction,
    },
};
use cnidarium::{
    ArcStateDeltaExt,
    RootHash,
    Snapshot,
    StateDelta,
    Storage,
};
use prost::Message as _;
use sha2::{
    Digest as _,
    Sha256,
};
use tendermint::{
    abci::{
        self,
        Event,
    },
    account,
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
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt,
    },
    component::Component as _,
    genesis::GenesisState,
    ibc::component::IbcComponent,
    proposal::{
        block_size_constraints::BlockSizeConstraints,
        commitment::{
            generate_rollup_datas_commitment,
            GeneratedCommitments,
        },
    },
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction,
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
#[derive(Debug)]
pub(crate) struct App {
    state: InterBlockState,

    // set to true when `prepare_proposal` is called, indicating we are the proposer for this
    // block. set to false when `process_proposal` is called, as it's called during the prevote
    // phase for that block.
    //
    // if true, `process_proposal` is not executed, as this means we are the proposer of that
    // block, and we have already executed the transactions for the block during
    // `prepare_proposal`, and re-executing them would cause failure.
    is_proposer: bool,

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
    execution_result: HashMap<[u8; 32], anyhow::Result<Vec<abci::Event>>>,

    /// set to `0` when `begin_block` is called, and set to `1` or `2` when
    /// `deliver_tx` is called for the first two times.
    /// this is a hack to allow the `sequence_actions_commitment` and `chain_ids_commitment`
    /// to pass `deliver_tx`, as they're the first two "tx"s delivered.
    ///
    /// when the app is fully updated to ABCI++, `begin_block`, `deliver_tx`,
    /// and `end_block` will all become one function `finalize_block`, so
    /// this will not be needed.
    processed_txs: u32,

    // proposer of the block being currently executed; set in begin_block
    // and cleared in end_block.
    // this is used only to determine who to transfer the block fees to
    // at the end of the block.
    current_proposer: Option<account::Id>,

    // builder of the current `SequencerBlock`.
    // initialized during `begin_block`, completed and written to state during `end_block`.
    current_sequencer_block_builder: Option<SequencerBlockBuilder>,
}

impl App {
    pub(crate) fn new(snapshot: Snapshot) -> Self {
        tracing::debug!("initializing App instance");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Self {
            state,
            is_proposer: false,
            executed_proposal_hash: Hash::default(),
            execution_result: HashMap::new(),
            processed_txs: 0,
            current_proposer: None,
            current_sequencer_block_builder: None,
        }
    }

    #[instrument(name = "App:init_chain", skip_all)]
    pub(crate) async fn init_chain(
        &mut self,
        genesis_state: GenesisState,
        genesis_validators: Vec<tendermint::validator::Update>,
        chain_id: String,
    ) -> anyhow::Result<()> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        crate::asset::initialize_native_asset(&genesis_state.native_asset_base_denomination);
        state_tx.put_native_asset_denom(&genesis_state.native_asset_base_denomination);
        state_tx.put_chain_id(chain_id);
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
        IbcComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on IbcComponent")?;

        state_tx.apply();
        Ok(())
    }

    fn update_state_for_new_round(&mut self, storage: &Storage) {
        // reset app state to latest committed state, in case of a round not being committed
        // but `self.state` was changed due to executing the previous round's data.
        //
        // if the previous round was committed, then the state stays the same.
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        // clear the cache of transaction execution results
        self.execution_result.clear();
        self.processed_txs = 0;
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
        self.is_proposer = true;
        self.update_state_for_new_round(&storage);

        let mut block_size_constraints = BlockSizeConstraints::new(
            usize::try_from(prepare_proposal.max_tx_bytes)
                .context("failed to convert max_tx_bytes to usize")?,
        );
        block_size_constraints
            .cometbft_checked_add(std::mem::size_of::<GeneratedCommitments>())
            .context(
                "commitment size should not be larger than the prepare proposal's max bytes",
            )?;

        let (signed_txs, txs_to_include) = self
            .execute_and_filter_block_data(prepare_proposal.txs, &mut block_size_constraints)
            .await
            .context("failed to execute and filter transactions in prepare_proposal")?;

        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in prepare_proposal")?;

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
        // if we didn't propose this block, `self.is_proposer` will be `false`, so
        // we will execute the block as normal.
        if self.is_proposer {
            debug!("skipping process_proposal as we are the proposer for this block");
            self.is_proposer = false;
            self.executed_proposal_hash = process_proposal.hash;
            return Ok(());
        }

        self.is_proposer = false;
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

        let signed_txs = self
            .execute_block_data(txs)
            .await
            .context("transactions failed to decode and execute")?;

        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in process_proposal")?;

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
    /// is stored in `self.execution_result`.
    ///
    /// Returns the transactions which were successfully decoded and executed
    /// in both their [`SignedTransaction`] and raw bytes form.
    ///
    /// Will filter transactions that are too large for the sequencer block or
    /// the cometBFT block.
    #[instrument(name = "App::execute_and_filter_block_data", skip_all, fields(
        tx_count = txs.len()
    ))]
    async fn execute_and_filter_block_data(
        &mut self,
        txs: Vec<bytes::Bytes>,
        block_size_constraints: &mut BlockSizeConstraints,
    ) -> anyhow::Result<(Vec<SignedTransaction>, Vec<bytes::Bytes>)> {
        let mut signed_txs: Vec<SignedTransaction> = Vec::with_capacity(txs.len());
        let mut validated_txs = Vec::with_capacity(txs.len());
        let mut excluded_tx_count: usize = 0;

        for tx in txs {
            let signed_tx = match signed_transaction_from_bytes(&tx) {
                Err(e) => {
                    debug!(
                        error = AsRef::<dyn std::error::Error>::as_ref(&e),
                        "failed to decode deliver tx payload to signed transaction; ignoring it",
                    );
                    excluded_tx_count += 1;
                    continue;
                }
                Ok(tx) => tx,
            };

            let tx_hash = Sha256::digest(&tx);

            let tx_sequence_data_bytes = signed_tx
                .unsigned_transaction()
                .actions
                .iter()
                .filter_map(Action::as_sequence)
                .fold(0usize, |acc, seq| acc + seq.data.len());

            // don't include tx if it would make the sequenced block data too large
            if !block_size_constraints.sequencer_has_space(tx_sequence_data_bytes) {
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    max_sequencer_bytes = block_size_constraints.max_sequencer,
                    included_data_bytes = block_size_constraints.current_sequencer,
                    tx_data_bytes = tx_sequence_data_bytes,
                    "excluding transaction: max block sequenced data limit reached"
                );
                excluded_tx_count += 1;
                continue;
            }

            // don't include tx if it would make the cometBFT block too large
            if !block_size_constraints.cometbft_has_space(tx.len()) {
                debug!(
                    transaction_hash = %telemetry::display::base64(&tx_hash),
                    max_cometbft_bytes = block_size_constraints.max_cometbft,
                    included_data_bytes = block_size_constraints.current_cometbft,
                    tx_data_bytes = tx.len(),
                    "excluding transactions: max cometBFT data limit reached"
                );
                excluded_tx_count += 1;
                continue;
            }

            // store transaction execution result
            match self.execute_and_store_signed_tx(&signed_tx).await {
                Ok(()) => {
                    block_size_constraints
                        .sequencer_checked_add(tx_sequence_data_bytes)
                        .context("error growing sequencer block size")?;
                    block_size_constraints
                        .cometbft_checked_add(tx.len())
                        .context("error growing cometBFT block size")?;
                    signed_txs.push(signed_tx);
                    validated_txs.push(tx);
                }
                Err(_e) => {
                    debug!(
                        transaction_hash = %telemetry::display::base64(&tx_hash),
                        "failed to execute transaction, not including in block"
                    );
                    excluded_tx_count += 1;
                }
            }
        }

        if excluded_tx_count > 0 {
            info!(
                excluded_tx_count = excluded_tx_count,
                included_tx_count = validated_txs.len(),
                "excluded transactions from block"
            );
        }

        Ok((signed_txs, validated_txs))
    }

    /// Executes the given transaction data, writing it to the app's `StateDelta`
    /// and stores the results in `self.execution_result`.
    ///
    /// Will throw error if any transaction fails to decode or execute as these
    /// transactions should've been filtered in prepare proposal.
    ///
    /// Returns the transactions which were [`SignedTransaction`] form.
    #[instrument(name = "App::execute_block_data", skip_all, fields(
        tx_count = txs.len()
    ))]
    async fn execute_block_data(
        &mut self,
        txs: VecDeque<bytes::Bytes>,
    ) -> anyhow::Result<Vec<SignedTransaction>, anyhow::Error> {
        let mut signed_txs = Vec::new();
        for tx in txs {
            // all txs in the proposal should be deserializable and executable,
            // if any txs were not deserializeable or executable, they would not have been
            // returned by `execute_block_data`
            let signed_tx = signed_transaction_from_bytes(&tx).context(
                "failed to decode tx bytes to signed transaction, should've been filtered out in \
                 prepare proposal",
            )?;
            self.execute_and_store_signed_tx(&signed_tx).await.context(
                "transaction failed during execution, should've been filtered out in prepare \
                 proposal",
            )?;
            signed_txs.push(signed_tx);
        }
        Ok(signed_txs)
    }

    /// Executes the given transaction data, writing it to the app's `StateDelta`
    /// and stores the results in `self.execution_result`.
    ///
    /// Returns the signed transaction if it successfully executed else an error.
    #[instrument(name = "App::execute_and_store_signed_tx", skip_all)]
    async fn execute_and_store_signed_tx(
        &mut self,
        signed_tx: &SignedTransaction,
    ) -> anyhow::Result<()> {
        // store transaction execution result, indexed by tx hash
        let tx_hash = Sha256::digest(signed_tx.to_raw().encode_to_vec());
        match self.deliver_tx(signed_tx.clone()).await {
            Ok(events) => {
                self.execution_result.insert(tx_hash.into(), Ok(events));
                Ok(())
            }
            Err(e) => {
                debug!(
                    transaction_hash = %telemetry::display::base64(&Sha256::digest(tx_hash)),
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed to execute transaction"
                );
                self.execution_result.insert(tx_hash.into(), Err(e));
                Err(anyhow!("failed to execute transaction"))
            }
        }
    }

    #[instrument(name = "App::begin_block", skip_all)]
    pub(crate) async fn begin_block(
        &mut self,
        begin_block: &abci::request::BeginBlock,
        storage: Storage,
    ) -> anyhow::Result<Vec<abci::Event>> {
        // clear the processed_txs count when beginning block execution
        self.processed_txs = 0;
        // set the current proposer
        self.current_proposer = Some(begin_block.header.proposer_address);

        self.current_sequencer_block_builder =
            Some(SequencerBlockBuilder::new(begin_block.header.clone()));

        // If we previously executed txs in a different proposal than is being processed reset
        // cached state changes.
        if self.executed_proposal_hash != begin_block.hash {
            self.update_state_for_new_round(&storage);
        }

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
        IbcComponent::begin_block(&mut arc_state_tx, begin_block)
            .await
            .context("failed to call begin_block on IbcComponent")?;

        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        Ok(self.apply(state_tx))
    }

    /// Called during the normal ABCI `deliver_tx` process, returning the results
    /// of transaction execution during the proposal phase.
    ///
    /// Since transaction execution now happens in the proposal phase, results
    /// are cached in the app and returned here during the usual ABCI block execution process.
    ///
    /// If the tx was not executed during the proposal phase it will be executed here.
    ///
    /// Note that the first two "transactions" in the block, which are the proposer-generated
    /// commitments, are ignored.
    #[instrument(name = "App::deliver_tx_after_proposal", skip_all, fields(
        tx_hash =  %telemetry::display::base64(&Sha256::digest(&tx.tx)),
    ))]
    pub(crate) async fn deliver_tx_after_proposal(
        &mut self,
        tx: abci::request::DeliverTx,
    ) -> Option<anyhow::Result<Vec<abci::Event>>> {
        self.current_sequencer_block_builder
            .as_mut()
            .expect(
                "begin_block must be called before deliver_tx, thus \
                 current_sequencer_block_builder must be set",
            )
            .push_transaction(tx.tx.to_vec());

        if self.processed_txs < 2 {
            self.processed_txs += 1;
            return Some(Ok(vec![]));
        }

        // When the hash is not empty, we have already executed and cached the results
        if !self.executed_proposal_hash.is_empty() {
            let tx_hash: [u8; 32] = sha2::Sha256::digest(&tx.tx).into();
            return self.execution_result.remove(&tx_hash);
        }

        let signed_tx = match signed_transaction_from_bytes(&tx.tx) {
            Err(e) => {
                // this is actually a protocol error, as only valid txs should be finalized
                debug!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed to decode deliver tx payload to signed transaction; ignoring it",
                );
                return None;
            }
            Ok(tx) => tx,
        };

        Some(self.deliver_tx(signed_tx).await)
    }

    /// Executes a signed transaction.
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
    ///
    /// Note that `begin_block` is now called *after* transaction execution.
    #[instrument(name = "App::deliver_tx", skip_all, fields(
        signed_transaction_hash = %telemetry::display::base64(&signed_tx.sha256_of_proto_encoding()),
        sender = %Address::from_verification_key(signed_tx.verification_key()),
    ))]
    pub(crate) async fn deliver_tx(
        &mut self,
        signed_tx: astria_core::sequencer::v1::SignedTransaction,
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
        end_block: &abci::request::EndBlock,
    ) -> anyhow::Result<abci::response::EndBlock> {
        use crate::api_state_ext::StateWriteExt as _;

        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, end_block)
            .await
            .context("failed to call end_block on AccountsComponent")?;
        AuthorityComponent::end_block(&mut arc_state_tx, end_block)
            .await
            .context("failed to call end_block on AuthorityComponent")?;
        IbcComponent::end_block(&mut arc_state_tx, end_block)
            .await
            .context("failed to call end_block on IbcComponent")?;

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
        let proposer = self
            .current_proposer
            .expect("current proposer must be set in `begin_block`");

        // convert tendermint id to astria address; this assumes they are
        // the same address, as they are both ed25519 keys
        let proposer_address = Address::try_from_slice(proposer.as_bytes())
            .context("failed to convert proposer tendermint id to astria address")?;
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
        self.current_proposer = None;

        // get and clear block deposits from state
        let deposits = self
            .state
            .get_block_deposits()
            .await
            .context("failed to get block deposits in end_block")?;
        self.current_sequencer_block_builder
            .as_mut()
            .expect(
                "begin_block must be called before end_block, thus \
                 current_sequencer_block_builder must be set",
            )
            .deposits(deposits);
        state_tx
            .clear_block_deposits()
            .await
            .context("failed to clear block deposits")?;

        // store the `SequencerBlock` in the state
        let sequencer_block = self
            .current_sequencer_block_builder
            .take()
            .expect(
                "begin_block must be called before end_block, thus \
                 current_sequencer_block_builder must be set",
            )
            .build()
            .context("failed to build sequencer block")?;
        state_tx
            .put_sequencer_block(sequencer_block)
            .context("failed to write sequencer block to state")?;

        let events = self.apply(state_tx);

        Ok(abci::response::EndBlock {
            validator_updates: validator_updates.into_tendermint_validator_updates(),
            events,
            ..Default::default()
        })
    }

    #[instrument(name = "App::commit", skip_all)]
    pub(crate) async fn commit(&mut self, storage: Storage) -> RootHash {
        // We need to extract the State we've built up to commit it.  Fill in a dummy state.
        let dummy_state = StateDelta::new(storage.latest_snapshot());

        let mut state = Arc::try_unwrap(std::mem::replace(&mut self.state, Arc::new(dummy_state)))
            .expect("we have exclusive ownership of the State at commit()");

        // store the storage version indexed by block height
        let new_version = storage.latest_version().wrapping_add(1);
        let height = state
            .get_block_height()
            .await
            .expect("block height must be set, as `begin_block` is always called before `commit`");
        state.put_storage_version_by_height(height, new_version);
        debug!(
            height,
            version = new_version,
            "stored storage version for height"
        );

        // Commit the pending writes, clearing the state.
        let app_hash = storage
            .commit(state)
            .await
            .expect("must be able to successfully commit to storage");
        tracing::debug!(
            app_hash = %telemetry::display::base64(&app_hash),
            "finished committing state",
        );

        // Get the latest version of the state, now that we've committed it.
        self.state = Arc::new(StateDelta::new(storage.latest_snapshot()));

        app_hash
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

#[derive(Debug)]
struct SequencerBlockBuilder {
    header: tendermint::block::Header,
    data: Vec<Vec<u8>>,
    deposits: HashMap<RollupId, Vec<Deposit>>,
}

impl SequencerBlockBuilder {
    fn new(header: tendermint::block::Header) -> Self {
        Self {
            header,
            data: Vec::new(),
            deposits: HashMap::new(),
        }
    }

    fn push_transaction(&mut self, tx: Vec<u8>) {
        self.data.push(tx);
    }

    fn deposits(&mut self, deposits: HashMap<RollupId, Vec<Deposit>>) {
        self.deposits = deposits;
    }

    fn build(self) -> anyhow::Result<SequencerBlock> {
        SequencerBlock::try_from_cometbft_header_and_data(self.header, self.data, self.deposits)
            .map_err(Into::into)
    }
}

fn signed_transaction_from_bytes(bytes: &[u8]) -> anyhow::Result<SignedTransaction> {
    let raw = raw::SignedTransaction::decode(bytes)
        .context("failed to decode protobuf to signed transaction")?;
    let tx = SignedTransaction::try_from_raw(raw)
        .context("failed to transform raw signed transaction to verified type")?;

    Ok(tx)
}

#[cfg(test)]
pub(crate) mod test_utils {
    use astria_core::sequencer::v1::{
        Address,
        ADDRESS_LEN,
    };
    use ed25519_consensus::SigningKey;

    // attempts to decode the given hex string into an address.
    pub(crate) fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address::from_array(arr)
    }

    pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
    pub(crate) const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";
    pub(crate) const CAROL_ADDRESS: &str = "60709e2d391864b732b4f0f51e387abb76743871";

    pub(crate) fn get_alice_signing_key_and_address() -> (SigningKey, Address) {
        // this secret key corresponds to ALICE_ADDRESS
        let alice_secret_bytes: [u8; 32] =
            hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
                .unwrap()
                .try_into()
                .unwrap();
        let alice_signing_key = SigningKey::from(alice_secret_bytes);
        let alice = Address::from_verification_key(alice_signing_key.verification_key());
        (alice_signing_key, alice)
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "mint")]
    use astria_core::sequencer::v1::transaction::action::MintAction;
    use astria_core::sequencer::v1::{
        asset,
        asset::DEFAULT_NATIVE_ASSET_DENOM,
        transaction::action::{
            IbcRelayerChangeAction,
            SequenceAction,
            SudoAddressChangeAction,
            TransferAction,
        },
        UnsignedTransaction,
    };
    use ed25519_consensus::SigningKey;
    use penumbra_ibc::params::IBCParameters;
    use tendermint::{
        abci::types::CommitInfo,
        block::{
            header::Version,
            Header,
            Height,
            Round,
        },
        AppHash,
        Time,
    };

    use super::*;
    use crate::{
        accounts::action::TRANSFER_FEE,
        app::test_utils::*,
        asset::get_native_asset,
        authority::state_ext::ValidatorSet,
        genesis::Account,
        ibc::state_ext::StateReadExt as _,
        sequence::calculate_fee,
        transaction::InvalidNonce,
    };

    fn default_genesis_accounts() -> Vec<Account> {
        vec![
            Account {
                address: address_from_hex_string(ALICE_ADDRESS),
                balance: 10u128.pow(19),
            },
            Account {
                address: address_from_hex_string(BOB_ADDRESS),
                balance: 10u128.pow(19),
            },
            Account {
                address: address_from_hex_string(CAROL_ADDRESS),
                balance: 10u128.pow(19),
            },
        ]
    }

    fn default_header() -> Header {
        Header {
            app_hash: AppHash::try_from(vec![]).unwrap(),
            chain_id: "test".to_string().try_into().unwrap(),
            consensus_hash: Hash::default(),
            data_hash: Some(Hash::try_from([0u8; 32].to_vec()).unwrap()),
            evidence_hash: Some(Hash::default()),
            height: Height::default(),
            last_block_id: None,
            last_commit_hash: Some(Hash::default()),
            last_results_hash: Some(Hash::default()),
            next_validators_hash: Hash::default(),
            proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
            time: Time::now(),
            validators_hash: Hash::default(),
            version: Version {
                app: 0,
                block: 0,
            },
        }
    }

    async fn initialize_app_with_storage(
        genesis_state: Option<GenesisState>,
        genesis_validators: Vec<tendermint::validator::Update>,
    ) -> (App, Storage) {
        let storage = cnidarium::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);

        let genesis_state = genesis_state.unwrap_or_else(|| GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: Address::from([0; 20]),
            ibc_sudo_address: Address::from([0; 20]),
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        });

        app.init_chain(genesis_state, genesis_validators, "test".to_string())
            .await
            .unwrap();

        (app, storage.clone())
    }

    async fn initialize_app(
        genesis_state: Option<GenesisState>,
        genesis_validators: Vec<tendermint::validator::Update>,
    ) -> App {
        let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;

        app
    }

    #[tokio::test]
    async fn app_genesis_and_init_chain() {
        let app = initialize_app(None, vec![]).await;
        assert_eq!(app.state.get_block_height().await.unwrap(), 0);

        for Account {
            address,
            balance,
        } in default_genesis_accounts()
        {
            assert_eq!(
                balance,
                app.state
                    .get_account_balance(address, get_native_asset().id())
                    .await
                    .unwrap(),
            );
        }

        assert_eq!(
            app.state.get_native_asset_denom().await.unwrap(),
            DEFAULT_NATIVE_ASSET_DENOM
        );
    }

    #[tokio::test]
    async fn app_begin_block() {
        let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

        let mut begin_block = abci::request::BeginBlock {
            header: default_header(),
            hash: Hash::default(),
            last_commit_info: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            byzantine_validators: vec![],
        };
        begin_block.header.height = 1u8.into();

        app.begin_block(&begin_block, storage).await.unwrap();
        assert_eq!(app.state.get_block_height().await.unwrap(), 1);
        assert_eq!(
            app.state.get_block_timestamp().await.unwrap(),
            begin_block.header.time
        );
    }

    #[tokio::test]
    async fn app_begin_block_remove_byzantine_validators() {
        use tendermint::{
            abci::types,
            validator,
        };

        let pubkey_a = tendermint::public_key::PublicKey::from_raw_ed25519(&[1; 32]).unwrap();
        let pubkey_b = tendermint::public_key::PublicKey::from_raw_ed25519(&[2; 32]).unwrap();

        let initial_validator_set = vec![
            validator::Update {
                pub_key: pubkey_a,
                power: 100u32.into(),
            },
            validator::Update {
                pub_key: pubkey_b,
                power: 1u32.into(),
            },
        ];

        let (mut app, storage) =
            initialize_app_with_storage(None, initial_validator_set.clone()).await;

        let misbehavior = types::Misbehavior {
            kind: types::MisbehaviorKind::Unknown,
            validator: types::Validator {
                address: tendermint::account::Id::from(pubkey_a)
                    .as_bytes()
                    .try_into()
                    .unwrap(),
                power: 0u32.into(),
            },
            height: Height::default(),
            time: Time::now(),
            total_voting_power: 101u32.into(),
        };

        let mut begin_block = abci::request::BeginBlock {
            header: default_header(),
            hash: Hash::default(),
            last_commit_info: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            byzantine_validators: vec![misbehavior],
        };
        begin_block.header.height = 1u8.into();

        app.begin_block(&begin_block, storage).await.unwrap();

        // assert that validator with pubkey_a is removed
        let validator_set = app.state.get_validator_set().await.unwrap();
        assert_eq!(validator_set.len(), 1);
        assert_eq!(
            validator_set.get(&pubkey_b.into()).unwrap().power,
            1u32.into()
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer() {
        let mut app = initialize_app(None, vec![]).await;

        // transfer funds from Alice to Bob
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let bob_address = address_from_hex_string(BOB_ADDRESS);
        let value = 333_333;
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                TransferAction {
                    to: bob_address,
                    amount: value,
                    asset_id: get_native_asset().id(),
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();

        let native_asset = get_native_asset().id();
        assert_eq!(
            app.state
                .get_account_balance(bob_address, native_asset)
                .await
                .unwrap(),
            value + 10u128.pow(19)
        );
        assert_eq!(
            app.state
                .get_account_balance(alice_address, native_asset)
                .await
                .unwrap(),
            10u128.pow(19) - (value + TRANSFER_FEE),
        );
        assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer_not_native_token() {
        use crate::accounts::state_ext::StateWriteExt as _;

        let mut app = initialize_app(None, vec![]).await;

        // create some asset to be transferred and update Alice's balance of it
        let asset = asset::Id::from_denom("test");
        let value = 333_333;
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let mut state_tx = StateDelta::new(app.state.clone());
        state_tx
            .put_account_balance(alice_address, asset, value)
            .unwrap();
        app.apply(state_tx);

        // transfer funds from Alice to Bob; use native token for fee payment
        let bob_address = address_from_hex_string(BOB_ADDRESS);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                TransferAction {
                    to: bob_address,
                    amount: value,
                    asset_id: asset,
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();

        let native_asset = get_native_asset().id();
        assert_eq!(
            app.state
                .get_account_balance(bob_address, native_asset)
                .await
                .unwrap(),
            10u128.pow(19), // genesis balance
        );
        assert_eq!(
            app.state
                .get_account_balance(bob_address, asset)
                .await
                .unwrap(),
            value, // transferred amount
        );

        assert_eq!(
            app.state
                .get_account_balance(alice_address, native_asset)
                .await
                .unwrap(),
            10u128.pow(19) - TRANSFER_FEE, // genesis balance - fee
        );
        assert_eq!(
            app.state
                .get_account_balance(alice_address, asset)
                .await
                .unwrap(),
            0, // 0 since all funds of `asset` were transferred
        );

        assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer_balance_too_low_for_fee() {
        use rand::rngs::OsRng;

        let mut app = initialize_app(None, vec![]).await;

        // create a new key; will have 0 balance
        let keypair = SigningKey::new(OsRng);
        let bob = address_from_hex_string(BOB_ADDRESS);

        // 0-value transfer; only fee is deducted from sender
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                TransferAction {
                    to: bob,
                    amount: 0,
                    asset_id: get_native_asset().id(),
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&keypair);
        let res = app
            .deliver_tx(signed_tx)
            .await
            .unwrap_err()
            .root_cause()
            .to_string();
        assert!(res.contains("insufficient funds"));
    }

    #[tokio::test]
    async fn app_deliver_tx_sequence() {
        let mut app = initialize_app(None, vec![]).await;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let data = b"hello world".to_vec();
        let fee = calculate_fee(&data).unwrap();

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                    data,
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        assert_eq!(
            app.state
                .get_account_balance(alice_address, get_native_asset().id())
                .await
                .unwrap(),
            10u128.pow(19) - fee,
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_invalid_fee_asset() {
        let mut app = initialize_app(None, vec![]).await;

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let data = b"hello world".to_vec();

        let fee_asset_id = asset::Id::from_denom("test");

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                    data,
                    fee_asset_id,
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[tokio::test]
    async fn app_deliver_tx_validator_update() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(&[1u8; 32]).unwrap();
        let update = tendermint::validator::Update {
            pub_key,
            power: 100u32.into(),
        };

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::ValidatorUpdate(update.clone())],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        let validator_updates = app.state.get_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(validator_updates.get(&pub_key.into()).unwrap(), &update);
    }

    #[tokio::test]
    async fn app_deliver_tx_ibc_relayer_change_addition() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
            ibc_params: IBCParameters::default(),
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![IbcRelayerChangeAction::Addition(alice_address).into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
        assert!(app.state.is_ibc_relayer(&alice_address).await.unwrap());
    }

    #[tokio::test]
    async fn app_deliver_tx_ibc_relayer_change_deletion() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![alice_address],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
            ibc_params: IBCParameters::default(),
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
        assert!(!app.state.is_ibc_relayer(&alice_address).await.unwrap());
    }

    #[tokio::test]
    async fn app_deliver_tx_ibc_relayer_change_invalid() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: Address::from([0; 20]),
            ibc_relayer_addresses: vec![alice_address],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
            ibc_params: IBCParameters::default(),
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[tokio::test]
    async fn app_deliver_tx_sudo_address_change() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let new_address = address_from_hex_string(BOB_ADDRESS);

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
                new_address,
            })],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        let sudo_address = app.state.get_sudo_address().await.unwrap();
        assert_eq!(sudo_address, new_address);
    }

    #[tokio::test]
    async fn app_deliver_tx_sudo_address_change_error() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let sudo_address = address_from_hex_string(CAROL_ADDRESS);

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: sudo_address,
            ibc_sudo_address: [0u8; 20].into(),
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
                new_address: alice_address,
            })],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let res = app
            .deliver_tx(signed_tx)
            .await
            .unwrap_err()
            .root_cause()
            .to_string();
        assert!(res.contains("signer is not the sudo key"));
    }

    #[tokio::test]
    async fn app_deliver_tx_fee_asset_change_addition() {
        use astria_core::sequencer::v1::transaction::action::FeeAssetChangeAction;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let new_asset = asset::Id::from_denom("test");

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Addition(
                new_asset,
            ))],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        assert!(app.state.is_allowed_fee_asset(new_asset).await.unwrap());
    }

    #[tokio::test]
    async fn app_deliver_tx_fee_asset_change_removal() {
        use astria_core::sequencer::v1::transaction::action::FeeAssetChangeAction;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let test_asset = asset::Denom::from_base_denom("test");

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![
                DEFAULT_NATIVE_ASSET_DENOM.to_owned().into(),
                test_asset.clone(),
            ],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
                test_asset.id(),
            ))],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        assert!(
            !app.state
                .is_allowed_fee_asset(test_asset.id())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_fee_asset_change_invalid() {
        use astria_core::sequencer::v1::transaction::action::FeeAssetChangeAction;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: alice_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
                get_native_asset().id(),
            ))],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let res = app
            .deliver_tx(signed_tx)
            .await
            .unwrap_err()
            .root_cause()
            .to_string();
        assert!(res.contains("cannot remove last allowed fee asset"));
    }

    #[tokio::test]
    async fn app_deliver_tx_init_bridge_account_ok() {
        use astria_core::sequencer::v1::transaction::action::InitBridgeAccountAction;

        use crate::bridge::init_bridge_account_action::INIT_BRIDGE_ACCOUNT_FEE;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();
        let action = InitBridgeAccountAction {
            rollup_id,
            asset_ids: vec![asset_id],
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);

        let before_balance = app
            .state
            .get_account_balance(alice_address, asset_id)
            .await
            .unwrap();
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
        assert_eq!(
            app.state
                .get_bridge_account_rollup_id(&alice_address)
                .await
                .unwrap()
                .unwrap(),
            rollup_id
        );
        assert_eq!(
            app.state
                .get_bridge_account_asset_ids(&alice_address)
                .await
                .unwrap(),
            vec![asset_id]
        );
        assert_eq!(
            app.state
                .get_account_balance(alice_address, asset_id)
                .await
                .unwrap(),
            before_balance - INIT_BRIDGE_ACCOUNT_FEE
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_init_bridge_account_empty_asset_ids() {
        use astria_core::sequencer::v1::transaction::action::InitBridgeAccountAction;

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();
        let action = InitBridgeAccountAction {
            rollup_id,
            asset_ids: vec![],
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[tokio::test]
    async fn app_deliver_tx_init_bridge_account_account_already_registered() {
        use astria_core::sequencer::v1::transaction::action::InitBridgeAccountAction;

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();
        let action = InitBridgeAccountAction {
            rollup_id,
            asset_ids: vec![asset_id],
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();

        let action = InitBridgeAccountAction {
            rollup_id,
            asset_ids: vec![asset_id],
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 1,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[tokio::test]
    async fn app_deliver_tx_bridge_lock_action_ok() {
        use astria_core::sequencer::v1::{
            block::Deposit,
            transaction::action::BridgeLockAction,
        };

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();

        let mut state_tx = StateDelta::new(app.state.clone());
        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[asset_id])
            .unwrap();
        app.apply(state_tx);

        let amount = 100;
        let action = BridgeLockAction {
            to: bridge_address,
            amount,
            asset_id,
            fee_asset_id: asset_id,
            destination_chain_address: "nootwashere".to_string(),
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);

        let alice_before_balance = app
            .state
            .get_account_balance(alice_address, asset_id)
            .await
            .unwrap();
        let bridge_before_balance = app
            .state
            .get_account_balance(bridge_address, asset_id)
            .await
            .unwrap();

        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
        assert_eq!(
            app.state
                .get_account_balance(alice_address, asset_id)
                .await
                .unwrap(),
            alice_before_balance - (amount + TRANSFER_FEE)
        );
        assert_eq!(
            app.state
                .get_account_balance(bridge_address, asset_id)
                .await
                .unwrap(),
            bridge_before_balance + amount
        );

        let expected_deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset_id,
            "nootwashere".to_string(),
        );

        let deposits = app.state.get_deposit_events(&rollup_id).await.unwrap();
        assert_eq!(deposits.len(), 1);
        assert_eq!(deposits[0], expected_deposit);
    }

    #[tokio::test]
    async fn app_deliver_tx_bridge_lock_action_invalid_for_eoa() {
        use astria_core::sequencer::v1::transaction::action::BridgeLockAction;

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        // don't actually register this address as a bridge address
        let bridge_address = Address::from([99; 20]);
        let asset_id = get_native_asset().id();

        let amount = 100;
        let action = BridgeLockAction {
            to: bridge_address,
            amount,
            asset_id,
            fee_asset_id: asset_id,
            destination_chain_address: "nootwashere".to_string(),
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer_invalid_to_bridge_account() {
        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let mut app = initialize_app(None, vec![]).await;

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();

        let mut state_tx = StateDelta::new(app.state.clone());
        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[asset_id])
            .unwrap();
        app.apply(state_tx);

        let amount = 100;
        let action = TransferAction {
            to: bridge_address,
            amount,
            asset_id,
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        assert!(app.deliver_tx(signed_tx).await.is_err());
    }

    #[cfg(feature = "mint")]
    #[tokio::test]
    async fn app_deliver_tx_mint() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: alice_address,
            ibc_sudo_address: [0u8; 20].into(),
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let bob_address = address_from_hex_string(BOB_ADDRESS);
        let value = 333_333;
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                MintAction {
                    to: bob_address,
                    amount: value,
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();

        assert_eq!(
            app.state
                .get_account_balance(bob_address, get_native_asset().id())
                .await
                .unwrap(),
            value + 10u128.pow(19)
        );
        assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn app_end_block_validator_updates() {
        use tendermint::validator;

        let pubkey_a = tendermint::public_key::PublicKey::from_raw_ed25519(&[1; 32]).unwrap();
        let pubkey_b = tendermint::public_key::PublicKey::from_raw_ed25519(&[2; 32]).unwrap();
        let pubkey_c = tendermint::public_key::PublicKey::from_raw_ed25519(&[3; 32]).unwrap();

        let initial_validator_set = vec![
            validator::Update {
                pub_key: pubkey_a,
                power: 100u32.into(),
            },
            validator::Update {
                pub_key: pubkey_b,
                power: 1u32.into(),
            },
        ];

        let mut app = initialize_app(None, initial_validator_set).await;
        app.current_proposer = Some(account::Id::try_from([0u8; 20].to_vec()).unwrap());

        let validator_updates = vec![
            validator::Update {
                pub_key: pubkey_a,
                power: 0u32.into(),
            },
            validator::Update {
                pub_key: pubkey_b,
                power: 100u32.into(),
            },
            validator::Update {
                pub_key: pubkey_c,
                power: 100u32.into(),
            },
        ];

        let mut state_tx = StateDelta::new(app.state.clone());
        state_tx
            .put_validator_updates(ValidatorSet::new_from_updates(validator_updates.clone()))
            .unwrap();
        app.apply(state_tx);

        let (_, sequencer_block_builder) = block_data_from_txs_no_sequence_actions(vec![]);
        app.current_sequencer_block_builder = Some(sequencer_block_builder);

        let resp = app
            .end_block(&abci::request::EndBlock {
                height: 1u32.into(),
            })
            .await
            .unwrap();
        // we only assert length here as the ordering of the updates is not guaranteed
        // and validator::Update does not implement Ord
        assert_eq!(resp.validator_updates.len(), validator_updates.len());

        // validator with pubkey_a should be removed (power set to 0)
        // validator with pubkey_b should be updated
        // validator with pubkey_c should be added
        let validator_set = app.state.get_validator_set().await.unwrap();
        assert_eq!(validator_set.len(), 2);
        let validator_b = validator_set.get(&pubkey_b.into()).unwrap();
        assert_eq!(validator_b.pub_key, pubkey_b);
        assert_eq!(validator_b.power, 100u32.into());
        let validator_c = validator_set.get(&pubkey_c.into()).unwrap();
        assert_eq!(validator_c.pub_key, pubkey_c);
        assert_eq!(validator_c.power, 100u32.into());
        assert_eq!(app.state.get_validator_updates().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn app_deliver_tx_invalid_nonce() {
        let mut app = initialize_app(None, vec![]).await;

        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        // create tx with invalid nonce 1
        let data = b"hello world".to_vec();
        let tx = UnsignedTransaction {
            nonce: 1,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                    data,
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let response = app.deliver_tx(signed_tx).await;

        // check that tx was not executed by checking nonce and balance are unchanged
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
        assert_eq!(
            app.state
                .get_account_balance(alice_address, get_native_asset().id())
                .await
                .unwrap(),
            10u128.pow(19),
        );

        assert_eq!(
            response
                .unwrap_err()
                .downcast_ref::<InvalidNonce>()
                .map(|nonce_err| nonce_err.0)
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn app_commit() {
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_address: Address::from([0; 20]),
            ibc_sudo_address: Address::from([0; 20]),
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        };

        let (mut app, storage) = initialize_app_with_storage(Some(genesis_state), vec![]).await;
        assert_eq!(app.state.get_block_height().await.unwrap(), 0);

        let native_asset = get_native_asset().id();
        for Account {
            address,
            balance,
        } in default_genesis_accounts()
        {
            assert_eq!(
                balance,
                app.state
                    .get_account_balance(address, native_asset)
                    .await
                    .unwrap()
            );
        }

        // commit should write the changes to the underlying storage
        app.commit(storage.clone()).await;
        let snapshot = storage.latest_snapshot();
        assert_eq!(snapshot.get_block_height().await.unwrap(), 0);
        for Account {
            address,
            balance,
        } in default_genesis_accounts()
        {
            assert_eq!(
                snapshot
                    .get_account_balance(address, native_asset)
                    .await
                    .unwrap(),
                balance
            );
        }
    }

    #[tokio::test]
    async fn app_transfer_block_fees_to_proposer() {
        let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let native_asset = get_native_asset().id();

        // transfer funds from Alice to Bob; use native token for fee payment
        let bob_address = address_from_hex_string(BOB_ADDRESS);
        let amount = 333_333;
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                TransferAction {
                    to: bob_address,
                    amount,
                    asset_id: native_asset,
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let (header, sequencer_block_builder) =
            block_data_from_txs_no_sequence_actions(vec![signed_tx.to_raw().encode_to_vec()]);

        let mut begin_block = abci::request::BeginBlock {
            header,
            hash: Hash::default(),
            last_commit_info: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            byzantine_validators: vec![],
        };
        begin_block.header.height = 1u8.into();
        let proposer_address =
            Address::try_from_slice(begin_block.header.proposer_address.as_bytes()).unwrap();

        app.begin_block(&begin_block, storage).await.unwrap();
        assert_eq!(app.state.get_block_height().await.unwrap(), 1);
        assert_eq!(
            app.state.get_block_timestamp().await.unwrap(),
            begin_block.header.time
        );
        assert_eq!(
            app.current_proposer.unwrap(),
            begin_block.header.proposer_address
        );

        app.deliver_tx(signed_tx).await.unwrap();

        app.current_sequencer_block_builder = Some(sequencer_block_builder);
        app.end_block(&abci::request::EndBlock {
            height: 1u32.into(),
        })
        .await
        .unwrap();

        // assert that transaction fees were transferred to the block proposer
        assert_eq!(
            app.state
                .get_account_balance(proposer_address, native_asset)
                .await
                .unwrap(),
            TRANSFER_FEE,
        );
        assert_eq!(app.state.get_block_fees().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn app_create_sequencer_block_with_sequenced_data_and_deposits() {
        use astria_core::sequencer::v1::{
            block::Deposit,
            transaction::action::BridgeLockAction,
        };

        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let asset_id = get_native_asset().id();

        let mut state_tx = StateDelta::new(app.state.clone());
        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[asset_id])
            .unwrap();
        app.apply(state_tx);

        let amount = 100;
        let lock_action = BridgeLockAction {
            to: bridge_address,
            amount,
            asset_id,
            fee_asset_id: asset_id,
            destination_chain_address: "nootwashere".to_string(),
        };
        let sequence_action = SequenceAction {
            rollup_id,
            data: b"hello world".to_vec(),
            fee_asset_id: asset_id,
        };
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![lock_action.into(), sequence_action.into()],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);

        let expected_deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset_id,
            "nootwashere".to_string(),
        );
        let deposits = HashMap::from_iter(vec![(rollup_id, vec![expected_deposit.clone()])]);

        let (header, commitments) =
            block_data_from_txs_with_sequence_actions_and_deposits(&[signed_tx.clone()], deposits);

        let begin_block = abci::request::BeginBlock {
            header,
            hash: Hash::default(),
            last_commit_info: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            byzantine_validators: vec![],
        };
        app.begin_block(&begin_block, storage).await.unwrap();

        // deliver the commitments and the signed tx to simulate the
        // action block execution and put them in the `app.current_sequencer_block_builder`
        app.deliver_tx_after_proposal(abci::request::DeliverTx {
            tx: commitments.rollup_datas_root.to_vec().into(),
        })
        .await
        .unwrap()
        .unwrap();
        app.deliver_tx_after_proposal(abci::request::DeliverTx {
            tx: commitments.rollup_ids_root.to_vec().into(),
        })
        .await
        .unwrap()
        .unwrap();
        app.deliver_tx_after_proposal(abci::request::DeliverTx {
            tx: signed_tx.to_raw().encode_to_vec().into(),
        })
        .await
        .unwrap()
        .unwrap();
        let deposits = app.state.get_deposit_events(&rollup_id).await.unwrap();
        assert_eq!(deposits.len(), 1);
        assert_eq!(deposits[0], expected_deposit);

        app.end_block(&abci::request::EndBlock {
            height: 1u32.into(),
        })
        .await
        .unwrap();

        // ensure deposits are cleared at the end of the block
        let deposit_events = app.state.get_deposit_events(&rollup_id).await.unwrap();
        assert_eq!(deposit_events.len(), 0);
    }

    fn block_data_from_txs_no_sequence_actions(
        txs: Vec<Vec<u8>>,
    ) -> (Header, SequencerBlockBuilder) {
        let empty_hash = merkle::Tree::from_leaves(Vec::<Vec<u8>>::new()).root();
        let mut block_data = vec![empty_hash.to_vec(), empty_hash.to_vec()];
        block_data.extend(txs);

        let data_hash = merkle::Tree::from_leaves(block_data.iter().map(Sha256::digest)).root();
        let mut header = default_header();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let mut sequencer_block_builder = SequencerBlockBuilder::new(header.clone());
        for tx in block_data {
            sequencer_block_builder.push_transaction(tx);
        }
        (header, sequencer_block_builder)
    }

    fn block_data_from_txs_with_sequence_actions_and_deposits(
        txs: &[SignedTransaction],
        deposits: HashMap<RollupId, Vec<Deposit>>,
    ) -> (Header, GeneratedCommitments) {
        let GeneratedCommitments {
            rollup_datas_root,
            rollup_ids_root,
        } = generate_rollup_datas_commitment(txs, deposits.clone());
        let mut block_data = vec![rollup_datas_root.to_vec(), rollup_ids_root.to_vec()];
        block_data.extend(txs.iter().map(|tx| tx.to_raw().encode_to_vec()));

        let data_hash = merkle::Tree::from_leaves(block_data.iter().map(Sha256::digest)).root();
        let mut header = default_header();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let mut sequencer_block_builder = SequencerBlockBuilder::new(header.clone());
        for tx in block_data {
            sequencer_block_builder.push_transaction(tx);
        }
        sequencer_block_builder.deposits = deposits;
        (
            header,
            GeneratedCommitments {
                rollup_datas_root,
                rollup_ids_root,
            },
        )
    }

    #[tokio::test]
    async fn app_cometbft_block_size_commitmemts_() {
        // ensure that the returned size of the commimtments is what is intended.
        // we need the size of whatever is being put into the cometBFT block
        assert_eq!(
            std::mem::size_of::<GeneratedCommitments>(),
            64,
            "expecting the size to be equal to the length of the commitments being appended to \
             the cometBFT block"
        );
    }

    #[tokio::test]
    async fn app_prepare_proposal_cometbft_max_bytes_overflow_ok() {
        let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

        // update storage with initalized genesis app state
        let intermediate_state = StateDelta::new(storage.latest_snapshot());
        let state = Arc::try_unwrap(std::mem::replace(
            &mut app.state,
            Arc::new(intermediate_state),
        ))
        .expect("we have exclusive ownership of the State at commit()");
        storage
            .commit(state)
            .await
            .expect("applying genesis state should be okay");

        // create txs which will cause cometBFT overflow
        let (alice_signing_key, _) = get_alice_signing_key_and_address();
        let tx_pass = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from([1u8; 32]),
                    data: vec![1u8; 100_000],
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        }
        .into_signed(&alice_signing_key);
        let tx_overflow = UnsignedTransaction {
            nonce: 1,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from([1u8; 32]),
                    data: vec![1u8; 100_000],
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        }
        .into_signed(&alice_signing_key);

        let txs: Vec<bytes::Bytes> = vec![
            tx_pass.to_raw().encode_to_vec().into(),
            tx_overflow.to_raw().encode_to_vec().into(),
        ];

        // send to prepare_proposal
        let prepare_args = abci::request::PrepareProposal {
            max_tx_bytes: 200_000,
            txs,
            local_last_commit: None,
            misbehavior: vec![],
            height: Height::default(),
            time: Time::now(),
            next_validators_hash: Hash::default(),
            proposer_address: account::Id::new([1u8; 20]),
        };

        let result = app
            .prepare_proposal(prepare_args, storage)
            .await
            .expect("too large transactions should not cause prepare proposal to fail");

        // see only first tx made it in
        assert_eq!(
            result.txs.len(),
            3,
            "total transaciton length should be three, including the two commitments and the one \
             tx that fit"
        );
    }
}
