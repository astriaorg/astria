use anyhow::Result;
use penumbra_component::Component;
use penumbra_storage::StateWrite;
use std::sync::Arc;
use tendermint::abci::{request::{BeginBlock, EndBlock}};

pub const ACCOUNTS_PREFIX: &str = "accounts";

/// The genesis state for the accounts component.
/// Contains a list of accounts with the given balance at genesis.
pub struct GenesisState {
    pub accounts: Vec<(String, u64)>,
}

pub struct AccountsComponent {}

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisState;

    async fn init_chain<S: StateWrite>(mut state: S, app_state: &Self::AppState) {
        for (address, balance) in &app_state.accounts {
            state.put_raw(
                format!("{}/{}", ACCOUNTS_PREFIX, address),
                balance.to_be_bytes().to_vec(),
            );
        }
    }

    async fn begin_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) {
        todo!()
    }

    async fn end_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) {
        todo!()
    }

    async fn end_epoch<S: StateWrite + 'static>(_state: &mut Arc<S>) -> Result<()> {
        Ok(())
    }
}
