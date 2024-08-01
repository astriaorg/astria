use std::sync::Arc;

use cnidarium::{
    StateRead,
    StateWrite,
};

/// This trait is a verbatim copy of [`cnidarium_component::ActionHandler`].
///
/// It's duplicated here because all actions are foreign types, forbidding
/// the the implementation of [`cnidarium_component::ActionHandler`] for these
/// types due to Rust orphan rules.
#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    type CheckStatelessContext: Clone + Send + Sync + 'static;
    async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;

    async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> anyhow::Result<()>;
}
