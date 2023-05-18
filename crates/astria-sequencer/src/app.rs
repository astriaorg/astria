use std::sync::Arc;

use anyhow::Result;
use penumbra_component::Component;
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

use crate::{
    accounts::component::AccountsComponent,
    state_ext::StateWriteExt as _,
    transaction::Transaction,
};

/// The application hash, used to verify the application state.
/// TODO: this may not be the same as the state root hash?
pub type AppHash = penumbra_storage::RootHash;

/// The inter-block state being written to by the application.
type InterBlockState = Arc<StateDelta<Snapshot>>;

/// The genesis state for the application.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct GenesisState {
    pub accounts: Vec<(String, u64)>,
}

impl Default for GenesisState {
    fn default() -> Self {
        Self {
            accounts: vec![],
        }
    }
}

/// The Sequencer application, written as a bundle of [`Component`]s.
#[derive(Clone)]
pub struct App {
    state: InterBlockState,
}

impl App {
    pub async fn new(snapshot: Snapshot) -> Result<Self> {
        tracing::debug!("initializing App instance");

        // We perform the `Arc` wrapping of `State` here to ensure
        // there should be no unexpected copies elsewhere.
        let state = Arc::new(StateDelta::new(snapshot));

        Ok(Self {
            state,
        })
    }

    pub async fn init_chain(&mut self, genesis_state: &GenesisState) -> Result<()> {
        tracing::debug!("initializing chain");

        // allocate to hard-coded accounts for testing
        let mut accounts = genesis_state.accounts.clone();
        accounts.append(&mut default_genesis_accounts());
        let genesis_state = GenesisState {
            accounts,
        };

        let mut state_tx = self
            .state
            .try_begin_transaction()
            .expect("failed to get state for init_chain");

        state_tx.put_block_height(0);

        // call init_chain on all components
        AccountsComponent::init_chain(&mut state_tx, &genesis_state).await;
        state_tx.apply();

        // TODO: call commit and return the app hash?
        Ok(())
    }

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
        Ok(vec![])
    }

    pub async fn end_block(&mut self, _end_block: &abci::request::EndBlock) -> Vec<abci::Event> {
        let state_tx = StateDelta::new(self.state.clone());
        let mut arc_state_tx = Arc::new(state_tx);

        // call end_block on all components
        AccountsComponent::end_block(&mut arc_state_tx, _end_block).await;
        let state_tx = Arc::try_unwrap(arc_state_tx)
            .expect("components should not retain copies of shared state");
        self.apply(state_tx)
    }

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

        let app_hash: AppHash = jmt_root.into();

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

fn default_genesis_accounts() -> Vec<(String, u64)> {
    vec![
        ("alice".to_string(), 10e18 as u64),
        ("bob".to_string(), 10e18 as u64),
        ("carol".to_string(), 10e18 as u64),
    ]
}
