use std::sync::Arc;

use anyhow::Result;
use penumbra_storage::{
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
    transaction::{
        ActionHandler as _,
        Transaction,
    },
};

/// The application hash, used to verify the application state.
/// TODO: this may not be the same as the state root hash?
pub(crate) type AppHash = penumbra_storage::RootHash;

/// The inter-block state being written to by the application.
type InterBlockState = Arc<StateDelta<Snapshot>>;

/// The Sequencer application, written as a bundle of [`Component`]s.
///
/// Note: this is called `App` because this is a Tendermint ABCI application,
/// and implements the state transition logic of the chain.
///
/// See also https://github.com/penumbra-zone/penumbra/blob/9cc2c644e05c61d21fdc7b507b96016ba6b9a935/app/src/app/mod.rs#L42.
#[derive(Clone, Debug)]
pub(crate) struct App {
    state: InterBlockState,
}

impl App {
    pub fn new(snapshot: Snapshot) -> Self {
        tracing::debug!("initializing App instance");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Self {
            state,
        }
    }

    #[instrument(name = "App:init_chain", skip(self))]
    pub async fn init_chain(&mut self, genesis_state: GenesisState) -> Result<()> {
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
    pub async fn begin_block(
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

        self.apply(state_tx)
    }

    #[instrument(name = "App:deliver_tx", skip(self))]
    pub async fn deliver_tx(&mut self, tx: &[u8]) -> Result<Vec<abci::Event>> {
        let tx = Transaction::from_bytes(tx)?;

        let tx2 = tx.clone();
        let stateless = tokio::spawn(async move { tx2.check_stateless() });
        let tx2 = tx.clone();
        let state2 = self.state.clone();
        let stateful = tokio::spawn(async move { tx2.check_stateful(&state2).await });

        stateless.await??;
        stateful.await??;

        // At this point, the stateful checks should have completed,
        // leaving us with exclusive access to the Arc<State>.
        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("state Arc should be present and unique");

        tx.execute(&mut state_tx).await?;
        state_tx.apply();

        let height = self
            .state
            .get_block_height()
            .await
            .expect("block height should be set");
        info!(?tx, ?height, "executed transaction");
        Ok(vec![])
    }

    #[instrument(name = "App:end_block", skip(self))]
    pub async fn end_block(&mut self, _end_block: &abci::request::EndBlock) -> Vec<abci::Event> {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, _end_block).await;
        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");
        self.apply(state_tx)
    }

    #[instrument(name = "App:commit", skip(self))]
    pub async fn commit(&mut self, storage: Storage) -> AppHash {
        // We need to extract the State we've built up to commit it.  Fill in a dummy state.
        let dummy_state = StateDelta::new(storage.latest_snapshot());
        let state = Arc::try_unwrap(std::mem::replace(&mut self.state, Arc::new(dummy_state)))
            .expect("we have exclusive ownership of the State at commit()");

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
            self,
            state_ext::StateReadExt as _,
            types::{
                Address,
                Balance,
            },
        },
        genesis::Account,
    };

    fn default_genesis_accounts() -> Vec<Account> {
        vec![
            Account {
                address: "alice".into(),
                balance: 10u128.pow(19).into(),
            },
            Account {
                address: "bob".into(),
                balance: 10u128.pow(19).into(),
            },
            Account {
                address: "carol".into(),
                balance: 10u128.pow(19).into(),
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
    async fn test_app_genesis_and_init_chain() {
        let storage = penumbra_storage::TempStorage::new()
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
                app.state.get_account_balance(&address).await.unwrap(),
            )
        }
    }

    #[tokio::test]
    async fn test_app_begin_block() {
        let storage = penumbra_storage::TempStorage::new()
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
    async fn test_app_deliver_tx() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        let genesis_state = GenesisState {
            accounts: default_genesis_accounts(),
        };
        app.init_chain(genesis_state).await.unwrap();

        // transfer funds from Alice to Bob
        let alice = Address::from("alice");
        let bob = Address::from("bob");
        let amount = Balance::from(333333);
        let nonce = 1.into();
        let tx = Transaction::AccountsTransaction(accounts::transaction::Transaction {
            from: alice.clone(),
            to: bob.clone(),
            amount,
            nonce,
        });
        let bytes = tx.to_bytes().unwrap();

        app.deliver_tx(&bytes).await.unwrap();
        assert_eq!(
            app.state.get_account_balance(&bob).await.unwrap(),
            amount + 10u128.pow(19)
        );
        assert_eq!(
            app.state.get_account_balance(&alice).await.unwrap(),
            Balance::from(10u128.pow(19)) - amount
        );
        assert_eq!(app.state.get_account_nonce(&bob).await.unwrap(), 0);
        assert_eq!(app.state.get_account_nonce(&alice).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_app_commit() {
        let storage = penumbra_storage::TempStorage::new()
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
                app.state.get_account_balance(&address).await.unwrap()
            )
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
                snapshot.get_account_balance(&address).await.unwrap(),
                balance
            )
        }
    }
}
