use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use penumbra_storage::StateWrite;
use tendermint::abci;

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

    /// Begins a new block, optionally inspecting the ABCI
    /// [`BeginBlock`](abci::request::BeginBlock) request.
    ///
    /// # Invariants
    ///
    /// The `&mut Arc<S>` allows the implementor to optionally share state with
    /// its subtasks.  The implementor SHOULD assume that when the method is
    /// called, `state.get_mut().is_some()`, i.e., the `Arc` is not shared.  The
    /// implementor MUST ensure that any clones of the `Arc` are dropped before
    /// it returns, so that `state.get_mut().is_some()` on completion.
    async fn begin_block<S: StateWrite + 'static>(
        state: &mut Arc<S>,
        begin_block: &abci::request::BeginBlock,
    ) -> Result<()>;

    /// Ends the block, optionally inspecting the ABCI
    /// [`EndBlock`](abci::request::EndBlock) request, and performing any batch
    /// processing.
    ///
    /// # Invariants
    ///
    /// This method should only be called after [`Component::begin_block`].
    /// No methods should be called following this method.
    ///
    /// The `&mut Arc<S>` allows the implementor to optionally share state with
    /// its subtasks.  The implementor SHOULD assume that when the method is
    /// called, `state.get_mut().is_some()`, i.e., the `Arc` is not shared.  The
    /// implementor MUST ensure that any clones of the `Arc` are dropped before
    /// it returns, so that `state.get_mut().is_some()` on completion.
    async fn end_block<S: StateWrite + 'static>(
        state: &mut Arc<S>,
        end_block: &abci::request::EndBlock,
    ) -> Result<()>;
}
