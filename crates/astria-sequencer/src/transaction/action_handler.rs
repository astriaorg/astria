use anyhow::Result;
use astria_core::primitive::v1::Address;
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};

#[async_trait]
pub(crate) trait ActionHandler {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }
    async fn check_stateful<S: StateRead + 'static>(
        &self,
        _state: &S,
        _from: Address,
    ) -> Result<()> {
        Ok(())
    }
    async fn execute<S: StateWrite>(&self, _state: &mut S, _from: Address) -> Result<()> {
        Ok(())
    }
}
