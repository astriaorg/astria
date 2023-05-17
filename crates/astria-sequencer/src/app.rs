use std::sync::Arc;

use anyhow::Result;
use penumbra_component::Component;
use penumbra_storage::{
    ArcStateDeltaExt,
    Snapshot,
    StateDelta,
    StateWrite,
    Storage,
};

use crate::accounts::AccountsComponent;

/// The inter-block state being written to by the application.
type InterBlockState = Arc<StateDelta<Snapshot>>;

/// The genesis state for the application.
pub struct GenesisState {
    pub accounts: Vec<(String, u64)>,
}

/// The Penumbra application, written as a bundle of [`Component`]s.
///
/// The [`App`] is not a [`Component`], but
/// it constructs the components and exposes a [`commit`](App::commit) that
/// commits the changes to the persistent storage and resets its subcomponents.
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
}
