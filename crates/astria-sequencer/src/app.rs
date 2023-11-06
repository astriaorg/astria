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
use penumbra_storage::{
    ArcStateDeltaExt,
    Snapshot,
    StateDelta,
    Storage,
};
use proto::{
    generated::sequencer::v1alpha1 as raw,
    native::sequencer::v1alpha1::{
        Address,
        SignedTransaction,
    },
    Message as _,
};
use sha2::Digest as _;
use tendermint::abci::{
    self,
    Event,
};
use tracing::{
    debug,
    info,
    instrument,
};

use crate::{
    accounts::component::AccountsComponent,
    app_hash::AppHash,
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
    component::Component,
    genesis::GenesisState,
    proposal::commitment::{
        generate_sequence_actions_commitment,
        GeneratedCommitments,
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

    // cache of results of executing of transactions in prepare_proposal or process_proposal.
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
            execution_result: HashMap::new(),
            processed_txs: 0,
        }
    }

    #[instrument(name = "App:init_chain", skip(self))]
    pub(crate) async fn init_chain(
        &mut self,
        genesis_state: GenesisState,
        genesis_validators: Vec<tendermint::validator::Update>,
    ) -> anyhow::Result<()> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        state_tx.put_block_height(0);

        // call init_chain on all components
        AccountsComponent::init_chain(&mut state_tx, &genesis_state)
            .await
            .context("failed to call init_chain on AccountsComponent")?;
        AuthorityComponent::init_chain(
            &mut state_tx,
            &AuthorityComponentAppState {
                authority_sudo_key: genesis_state.authority_sudo_key,
                genesis_validators,
            },
        )
        .await
        .context("failed to call init_chain on AuthorityComponent")?;
        state_tx.apply();
        Ok(())
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions.
    ///
    /// This is required so that a rollup can easily verify that the transactions it
    /// receives are correct (ie. we actually included in a sequencer block, and none
    /// are missing)
    /// It puts this special "commitment" as the first transaction in a block.
    /// When other validators receive the block, they know the first transaction is
    /// supposed to be the commitment, and verifies that is it correct.
    #[instrument(name = "App::prepare_proposal", skip(self))]
    pub(crate) async fn prepare_proposal(
        &mut self,
        prepare_proposal: abci::request::PrepareProposal,
    ) -> abci::response::PrepareProposal {
        // clear the cache of transaction execution results
        self.execution_result.clear();
        self.processed_txs = 0;
        self.is_proposer = true;

        let (signed_txs, txs_to_include) = self.execute_block_data(prepare_proposal.txs).await;

        // generate commitment to sequence::Actions and commitment to the chain IDs included in the
        // sequence::Actions
        let res = generate_sequence_actions_commitment(&signed_txs);

        abci::response::PrepareProposal {
            txs: res.into_transactions(txs_to_include),
        }
    }

    /// Generates a commitment to the `sequence::Actions` in the block's transactions
    /// and ensures it matches the commitment created by the proposer, which
    /// should be the first transaction in the block.
    #[instrument(name = "App::process_proposal", skip(self))]
    pub(crate) async fn process_proposal(
        &mut self,
        process_proposal: abci::request::ProcessProposal,
    ) -> anyhow::Result<()> {
        // if we proposed this block (ie. prepare_proposal was called directly before this), then
        // we skip execution for this `process_proposal` call.
        //
        // if we didn't propose this block, `self.is_proposer` will be `false`, so
        // we will execute the block as normal.
        if self.is_proposer {
            debug!("skipping process_proposal as we are the proposer for this block");
            self.is_proposer = false;
            return Ok(());
        }

        self.is_proposer = false;

        // clear the cache of transaction execution results
        self.execution_result.clear();
        self.processed_txs = 0;

        let mut txs = VecDeque::from(process_proposal.txs);
        let received_action_commitment: [u8; 32] = txs
            .pop_front()
            .context("no transaction commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| anyhow!("transaction commitment must be 32 bytes"))?;

        let received_chain_ids_commitment: [u8; 32] = txs
            .pop_front()
            .context("no chain IDs commitment in proposal")?
            .to_vec()
            .try_into()
            .map_err(|_| anyhow!("chain IDs commitment must be 32 bytes"))?;

        let expected_txs_len = txs.len();

        let (signed_txs, txs_to_include) = self.execute_block_data(txs.into()).await;

        // all txs in the proposal should be deserializable and executable
        // if any txs were not deserializeable or executable, they would not have been
        // returned by `execute_block_data`, thus the length of `txs_to_include`
        // will be shorter than that of `txs`.
        ensure!(
            txs_to_include.len() == expected_txs_len,
            "transactions to be included do not match expected",
        );

        let GeneratedCommitments {
            sequence_actions_commitment: expected_action_commitment,
            chain_ids_commitment: expected_chain_ids_commitment,
        } = generate_sequence_actions_commitment(&signed_txs);
        ensure!(
            received_action_commitment == expected_action_commitment,
            "transaction commitment does not match expected",
        );

        ensure!(
            received_chain_ids_commitment == expected_chain_ids_commitment,
            "chain IDs commitment does not match expected",
        );

        Ok(())
    }

    /// Executes the given transaction data, writing it to the app's `StateDelta`.
    ///
    /// The result of execution of every transaction which is successfully decoded
    /// is stored in `self.execution_result`.
    ///
    /// Returns the transactions which were successfully decoded and executed
    /// in both their [`SignedTransaction`] and raw bytes form.
    async fn execute_block_data(
        &mut self,
        txs: Vec<bytes::Bytes>,
    ) -> (Vec<SignedTransaction>, Vec<bytes::Bytes>) {
        let mut signed_txs = Vec::with_capacity(txs.len());
        let mut validated_txs = Vec::with_capacity(txs.len());

        for tx in txs {
            let Some(signed_tx) = raw::SignedTransaction::decode(&*tx)
            .map_err(|err| {
                debug!(error = ?err, "failed to deserialize bytes as a signed transaction");
                err
            })
            .ok()
            .and_then(|raw_tx| SignedTransaction::try_from_raw(raw_tx)
                .map_err(|err| {
                    debug!(error = ?err, "failed to convert raw signed transaction to native signed transaction");
                    err
                })
                .ok()
            ) else {
                continue;
            };

            // store transaction execution result, indexed by tx hash
            let tx_hash = sha2::Sha256::digest(&tx);
            match self.deliver_tx(signed_tx.clone()).await {
                Ok(events) => {
                    self.execution_result.insert(tx_hash.into(), Ok(events));
                    signed_txs.push(signed_tx);
                    validated_txs.push(tx);
                }
                Err(e) => {
                    debug!(?tx_hash, error = ?e, "failed to execute transaction, not including in block");
                    self.execution_result.insert(tx_hash.into(), Err(e));
                }
            }
        }

        (signed_txs, validated_txs)
    }

    #[instrument(name = "App::begin_block", skip(self))]
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
    /// Note that the first two "transactions" in the block, which are the proposer-generated
    /// commitments, are ignored.
    #[instrument(name = "App::deliver_tx_after_execution", skip(self))]
    pub(crate) fn deliver_tx_after_execution(
        &mut self,
        tx_hash: &[u8; 32],
    ) -> Option<anyhow::Result<Vec<abci::Event>>> {
        if self.processed_txs < 2 {
            self.processed_txs += 1;
            return Some(Ok(vec![]));
        }

        self.execution_result.remove(tx_hash)
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
    #[instrument(name = "App::deliver_tx", skip(self))]
    pub(crate) async fn deliver_tx(
        &mut self,
        signed_tx: proto::native::sequencer::v1alpha1::SignedTransaction,
    ) -> anyhow::Result<Vec<abci::Event>> {
        let signed_tx_2 = signed_tx.clone();
        let stateless = tokio::spawn(async move { transaction::check_stateless(&signed_tx_2) });
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
        state_tx.apply();

        // note: deliver_tx is now called (internally) before begin_block,
        // so increment the logged height by 1.
        let height = self.state.get_block_height().await.expect(
            "block height must be set, as `begin_block` is always called before `deliver_tx`",
        );
        info!(
            ?signed_tx,
            height = height + 1,
            sender = %Address::from_verification_key(signed_tx.verification_key()),
            "executed transaction"
        );
        Ok(vec![])
    }

    #[instrument(name = "App::end_block", skip(self))]
    pub(crate) async fn end_block(
        &mut self,
        end_block: &abci::request::EndBlock,
    ) -> anyhow::Result<abci::response::EndBlock> {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, end_block)
            .await
            .context("failed to call end_block on AccountsComponent")?;
        AuthorityComponent::end_block(&mut arc_state_tx, end_block)
            .await
            .context("failed to call end_block on AuthorityComponent")?;

        // gather and return validator updates
        let validator_updates = self
            .state
            .get_validator_updates()
            .await
            .expect("failed getting validator updates");

        let mut state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        // clear validator updates
        state_tx.clear_validator_updates();

        let events = self.apply(state_tx);

        Ok(abci::response::EndBlock {
            validator_updates: validator_updates.into_tendermint_validator_updates(),
            events,
            ..Default::default()
        })
    }

    #[instrument(name = "App::commit", skip(self))]
    pub(crate) async fn commit(&mut self, storage: Storage) -> AppHash {
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
        let jmt_root = storage
            .commit(state)
            .await
            .expect("must be able to successfully commit to storage");

        let app_hash = AppHash::from(jmt_root);
        tracing::debug!(?app_hash, "finished committing state");

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

#[cfg(test)]
mod test {
    use ed25519_consensus::SigningKey;
    #[cfg(feature = "mint")]
    use proto::native::sequencer::v1alpha1::MintAction;
    use proto::native::sequencer::v1alpha1::{
        Address,
        SequenceAction,
        SudoAddressChangeAction,
        TransferAction,
        UnsignedTransaction,
        ADDRESS_LEN,
    };
    use tendermint::{
        abci::types::CommitInfo,
        account,
        block::{
            header::Version,
            Header,
            Height,
            Round,
        },
        AppHash,
        Hash,
        Time,
    };

    use super::*;
    use crate::{
        accounts::{
            action::TRANSFER_FEE,
            state_ext::StateReadExt as _,
        },
        authority::state_ext::ValidatorSet,
        genesis::Account,
        sequence::calculate_fee,
        transaction::InvalidNonce,
    };

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address::from_array(arr)
    }

    const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";
    const CAROL_ADDRESS: &str = "60709e2d391864b732b4f0f51e387abb76743871";

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
            data_hash: Some(Hash::default()),
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

    async fn initialize_app(
        genesis_state: Option<GenesisState>,
        genesis_validators: Vec<tendermint::validator::Update>,
    ) -> App {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);

        let genesis_state = genesis_state.unwrap_or_else(|| GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_key: Address::from([0; 20]),
        });

        app.init_chain(genesis_state, genesis_validators)
            .await
            .unwrap();
        app
    }

    fn get_alice_signing_key_and_address() -> (SigningKey, Address) {
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
                app.state.get_account_balance(address).await.unwrap(),
            );
        }
    }

    #[tokio::test]
    async fn app_begin_block() {
        let mut app = initialize_app(None, vec![]).await;

        let mut begin_block = abci::request::BeginBlock {
            header: default_header(),
            hash: Hash::default(),
            last_commit_info: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            byzantine_validators: vec![],
        };
        begin_block.header.height = Height::try_from(1u8).unwrap();

        app.begin_block(&begin_block).await.unwrap();
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

        let mut app = initialize_app(None, initial_validator_set.clone()).await;

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
        begin_block.header.height = Height::try_from(1u8).unwrap();

        app.begin_block(&begin_block).await.unwrap();

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
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(
            app.state.get_account_balance(bob_address).await.unwrap(),
            value + 10u128.pow(19)
        );
        assert_eq!(
            app.state.get_account_balance(alice_address).await.unwrap(),
            10u128.pow(19) - (value + TRANSFER_FEE),
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
                    chain_id: b"testchainid".to_vec(),
                    data,
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        assert_eq!(
            app.state.get_account_balance(alice_address).await.unwrap(),
            10u128.pow(19) - fee,
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_validator_update() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_key: alice_address,
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(&[1u8; 32]).unwrap();
        let update = tendermint::validator::Update {
            pub_key,
            power: 100u32.into(),
        };

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![proto::native::sequencer::v1alpha1::Action::ValidatorUpdate(
                update.clone(),
            )],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        app.deliver_tx(signed_tx).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

        let validator_updates = app.state.get_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(validator_updates.get(&pub_key.into()).unwrap(), &update);
    }

    #[tokio::test]
    async fn app_deliver_tx_sudo_address_change() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_key: alice_address,
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let new_address = address_from_hex_string(BOB_ADDRESS);

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                proto::native::sequencer::v1alpha1::Action::SudoAddressChange(
                    SudoAddressChangeAction {
                        new_address,
                    },
                ),
            ],
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
            authority_sudo_key: sudo_address,
        };
        let mut app = initialize_app(Some(genesis_state), vec![]).await;

        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                proto::native::sequencer::v1alpha1::Action::SudoAddressChange(
                    SudoAddressChangeAction {
                        new_address: alice_address,
                    },
                ),
            ],
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

    #[cfg(feature = "mint")]
    #[tokio::test]
    async fn app_deliver_tx_mint() {
        let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_key: alice_address,
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
            app.state.get_account_balance(bob_address).await.unwrap(),
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
                    chain_id: b"testchainid".to_vec(),
                    data,
                }
                .into(),
            ],
        };

        let signed_tx = tx.into_signed(&alice_signing_key);
        let response = app.deliver_tx(signed_tx).await;

        // check that tx was not executed by checking nonce and balance are unchanged
        assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
        assert_eq!(
            app.state.get_account_balance(alice_address).await.unwrap(),
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
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
            authority_sudo_key: Address::from([0; 20]),
        };

        app.init_chain(genesis_state, vec![]).await.unwrap();
        assert_eq!(app.state.get_block_height().await.unwrap(), 0);

        for Account {
            address,
            balance,
        } in default_genesis_accounts()
        {
            assert_eq!(
                balance,
                app.state.get_account_balance(address).await.unwrap()
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
                snapshot.get_account_balance(address).await.unwrap(),
                balance
            );
        }
    }
}
