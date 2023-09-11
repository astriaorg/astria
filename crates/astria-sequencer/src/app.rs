use std::sync::Arc;

use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::Address;
use storage::{
    ArcStateDeltaExt,
    Snapshot,
    StateDelta,
    Storage,
};
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
    component::Component,
    genesis::GenesisState,
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction,
};

/// The application hash, used to verify the application state.
/// FIXME(https://github.com/astriaorg/astria/issues/348): `AppHash` exists as a type alias
/// here and as a proper type [`crate::app_hash::AppHash`]. That's confusing.
pub(crate) type AppHash = storage::RootHash;

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
#[derive(Clone, Debug)]
pub(crate) struct App {
    state: InterBlockState,

    /// set to `true` when `begin_block` is called, and set to `false` when
    /// `deliver_tx` is called for the first time.
    /// this is a hack to allow the `action_tree_root` to pass `deliver_tx`,
    /// as it's the first "tx" delivered.
    /// when the app is fully updated to ABCI++, `begin_block`, `deliver_tx`,
    /// and `end_block` will all become one function `finalize_block`, so
    /// this will not be needed.
    has_block_just_begun: bool,
}

impl App {
    pub(crate) fn new(snapshot: Snapshot) -> Self {
        tracing::debug!("initializing App instance");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Self {
            state,
            has_block_just_begun: false,
        }
    }

    #[instrument(name = "App:init_chain", skip(self))]
    pub(crate) async fn init_chain(&mut self, genesis_state: GenesisState) -> Result<()> {
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should not be referenced elsewhere");

        state_tx.put_block_height(0);

        // call init_chain on all components
        AccountsComponent::init_chain(&mut state_tx, &genesis_state).await?;
        state_tx.apply();

        // TODO: call commit and return the app hash?
        Ok(())
    }

    #[instrument(name = "App:begin_block", skip(self))]
    pub(crate) async fn begin_block(
        &mut self,
        begin_block: &abci::request::BeginBlock,
    ) -> Vec<abci::Event> {
        let mut state_tx = StateDelta::new(self.state.clone());

        // store the block height
        state_tx.put_block_height(begin_block.header.height.into());
        // store the block time
        state_tx.put_block_timestamp(begin_block.header.time);

        // call begin_block on all components
        let mut arc_state_tx = Arc::new(state_tx);
        AccountsComponent::begin_block(&mut arc_state_tx, begin_block).await;

        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");

        self.has_block_just_begun = true;
        self.apply(state_tx)
    }

    #[instrument(name = "App:deliver_tx", skip(self))]
    pub(crate) async fn deliver_tx(&mut self, tx: &[u8]) -> Result<Vec<abci::Event>> {
        use proto::{
            generated::sequencer::v1alpha1 as raw,
            native::sequencer::v1alpha1::SignedTransaction,
            Message as _,
        };
        if self.has_block_just_begun {
            ensure!(tx.len() == 32);
            self.has_block_just_begun = false;
            return Ok(vec![]);
        }

        let raw_signed_tx = raw::SignedTransaction::decode(tx)
            .context("failed deserializing raw signed protobuf transaction from bytes")?;
        let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
            .context("failed creating a verified signed transaction from the raw proto type")?;

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

        let height = self.state.get_block_height().await.expect(
            "block height must be set, as `begin_block` is always called before `deliver_tx`",
        );
        info!(
            ?tx,
            height,
            sender = %Address::from_verification_key(signed_tx.verification_key()),
            "executed transaction"
        );
        Ok(vec![])
    }

    #[instrument(name = "App:end_block", skip(self))]
    pub(crate) async fn end_block(
        &mut self,
        end_block: &abci::request::EndBlock,
    ) -> Vec<abci::Event> {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, end_block).await;
        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");
        self.apply(state_tx)
    }

    #[instrument(name = "App:commit", skip(self))]
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

        let app_hash: AppHash = jmt_root;
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
    use proto::{
        native::sequencer::v1alpha1::{
            Address,
            SequenceAction,
            TransferAction,
            UnsignedTransaction,
            ADDRESS_LEN,
        },
        Message as _,
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
        genesis::Account,
        sequence::calculate_fee,
    };

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address::from_array(arr)
    }

    // generated with test `generate_default_keys()`
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

    #[tokio::test]
    async fn app_genesis_and_init_chain() {
        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };
        app.init_chain(genesis_state).await.unwrap();
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
        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: vec![],
        };
        app.init_chain(genesis_state).await.unwrap();

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

        app.begin_block(&begin_block).await;
        assert_eq!(app.state.get_block_height().await.unwrap(), 1);
        assert_eq!(
            app.state.get_block_timestamp().await.unwrap(),
            begin_block.header.time
        );
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer() {
        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };
        app.init_chain(genesis_state).await.unwrap();

        // transfer funds from Alice to Bob
        // this secret key corresponds to ALICE_ADDRESS
        let alice_secret_bytes: [u8; 32] =
            hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
                .unwrap()
                .try_into()
                .unwrap();
        let alice_keypair = SigningKey::from(alice_secret_bytes);

        let alice = address_from_hex_string(ALICE_ADDRESS);
        let bob = address_from_hex_string(BOB_ADDRESS);
        let value = 333_333;
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                TransferAction {
                    to: bob,
                    amount: value,
                }
                .into(),
            ],
        };
        let signed_tx = tx.into_signed(&alice_keypair);
        let bytes = signed_tx.into_raw().encode_to_vec();

        app.deliver_tx(&bytes).await.unwrap();
        assert_eq!(
            app.state.get_account_balance(bob).await.unwrap(),
            value + 10u128.pow(19)
        );
        assert_eq!(
            app.state.get_account_balance(alice).await.unwrap(),
            10u128.pow(19) - (value + TRANSFER_FEE),
        );
        assert_eq!(app.state.get_account_nonce(bob).await.unwrap(), 0);
        assert_eq!(app.state.get_account_nonce(alice).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn app_deliver_tx_transfer_balance_too_low_for_fee() {
        use rand::rngs::OsRng;

        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };
        app.init_chain(genesis_state).await.unwrap();

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
        let bytes = signed_tx.into_raw().encode_to_vec();
        let res = app
            .deliver_tx(&bytes)
            .await
            .unwrap_err()
            .root_cause()
            .to_string();
        assert!(res.contains("insufficient funds"));
    }

    #[tokio::test]
    async fn app_deliver_tx_sequence() {
        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };
        app.init_chain(genesis_state).await.unwrap();

        // this secret key corresponds to ALICE_ADDRESS
        let alice_secret_bytes: [u8; 32] =
            hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
                .unwrap()
                .try_into()
                .unwrap();
        let alice_signing_key = SigningKey::from(alice_secret_bytes);
        let alice = Address::from_verification_key(alice_signing_key.verification_key());

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
        let bytes = signed_tx.into_raw().encode_to_vec();

        app.deliver_tx(&bytes).await.unwrap();
        assert_eq!(app.state.get_account_nonce(alice).await.unwrap(), 1);

        assert_eq!(
            app.state.get_account_balance(alice).await.unwrap(),
            10u128.pow(19) - fee,
        );
    }

    #[tokio::test]
    async fn app_commit() {
        let storage = storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };

        app.init_chain(genesis_state).await.unwrap();
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
