use std::sync::Arc;

use anyhow::Result;
use penumbra_component::Component;
use penumbra_storage::{
    ArcStateDeltaExt,
    Snapshot,
    StateDelta,
    Storage,
};
use tendermint::abci;

use crate::accounts::AccountsComponent;

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
        let mut state = self
            .state
            .try_begin_transaction()
            .expect("failed to get state for init_chain");
        AccountsComponent::init_chain(&mut state, genesis_state).await;
        Ok(())
    }

    pub async fn begin_block(
        &mut self,
        _begin_block: &abci::request::BeginBlock,
    ) -> Vec<abci::Event> {
        todo!()
    }

    pub async fn deliver_tx(&mut self, _tx: &[u8]) -> Result<Vec<abci::Event>> {
        todo!()
    }

    pub async fn end_block(&mut self, _end_block: &abci::request::EndBlock) -> Vec<abci::Event> {
        todo!()
    }

    pub async fn commit(&mut self, _storage: Storage) -> AppHash {
        todo!()
    }
}
