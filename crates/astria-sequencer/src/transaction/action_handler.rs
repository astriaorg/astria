use anyhow::Result;
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};

#[async_trait]
pub(crate) trait ActionHandler {
    fn check_stateless(&self) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()>;
}
