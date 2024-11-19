use std::sync::Arc;

use astria_eyre::eyre::Result;
use async_trait::async_trait;
use cnidarium::StateWrite;
use tendermint::{
    abci::types,
    account,
    block,
    chain,
    AppHash,
    Hash,
    Time,
};

pub(crate) struct PrepareStateInfo {
    pub(crate) app_hash: AppHash,
    pub(crate) byzantine_validators: Vec<types::Misbehavior>,
    pub(crate) chain_id: chain::Id,
    pub(crate) height: block::Height,
    pub(crate) next_validators_hash: Hash,
    pub(crate) proposer_address: account::Id,
    pub(crate) time: Time,
}

/// A component of the Sequencer application.
/// Based off Penumbra's [`Component`], but with modifications.
#[async_trait]
pub(crate) trait Component {
    /// A serialized representation of the component's application state,
    /// passed in to [`Component::init_chain`].
    type AppState;

    /// Performs initialization, given the genesis state.
    ///
    /// This method is called once per chain, and should only perform
    /// writes, since the backing tree for the [`State`] will
    /// be empty.
    async fn init_chain<S: StateWrite>(state: S, app_state: &Self::AppState) -> Result<()>;

    /// Makes necessary state changes for the given component at the start of the block.
    ///
    /// # Invariants
    ///
    /// The `&mut Arc<S>` allows the implementor to optionally share state with
    /// its subtasks.  The implementor SHOULD assume that when the method is
    /// called, `state.get_mut().is_some()`, i.e., the `Arc` is not shared.  The
    /// implementor MUST ensure that any clones of the `Arc` are dropped before
    /// it returns, so that `state.get_mut().is_some()` on completion.
    async fn prepare_state_for_tx_execution<S: StateWrite + 'static>(
        state: &mut Arc<S>,
        prepare_state_for_tx_execution: &PrepareStateInfo,
    ) -> Result<()>;

    /// Handles necessary state changes for the given component after transaction execution, ending
    /// the block.
    ///
    /// # Invariants
    ///
    /// This method should only be called after [`Component::prepare_state_for_tx_execution`].
    /// No methods should be called following this method.
    ///
    /// The `&mut Arc<S>` allows the implementor to optionally share state with
    /// its subtasks.  The implementor SHOULD assume that when the method is
    /// called, `state.get_mut().is_some()`, i.e., the `Arc` is not shared.  The
    /// implementor MUST ensure that any clones of the `Arc` are dropped before
    /// it returns, so that `state.get_mut().is_some()` on completion.
    async fn handle_post_tx_execution<S: StateWrite + 'static>(state: &mut Arc<S>) -> Result<()>;
}
