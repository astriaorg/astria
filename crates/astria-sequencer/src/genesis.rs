use astria_eyre::eyre::Result;
use async_trait::async_trait;
use cnidarium::StateWrite;

/// A component of the Sequencer application's implementation for genesis.
#[async_trait]
pub(crate) trait Genesis {
    /// A serialized representation of the component's application state,
    /// passed in to [`Genesis::init_chain`].
    type AppState;

    /// Performs initialization, given the genesis state.
    ///
    /// This method is called once per chain, and should only perform
    /// writes, since the backing tree for the [`State`] will
    /// be empty.
    async fn init_chain<S: StateWrite>(state: S, app_state: &Self::AppState) -> Result<()>;
}
