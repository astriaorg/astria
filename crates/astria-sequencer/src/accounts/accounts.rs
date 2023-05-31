use std::sync::Arc;

use anyhow::Result;
use penumbra_component::Component;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};

use crate::app::GenesisState;

pub struct AccountsComponent {}

impl AccountsComponent {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisState;

    async fn init_chain<S: StateWrite>(mut state: S, app_state: &Self::AppState) {
        for (address, balance) in &app_state.accounts {
            state.put_raw(storage_key(address), balance.to_be_bytes().to_vec());
        }
    }

    async fn begin_block<S: StateWrite + 'static>(_state: &mut Arc<S>, _begin_block: &BeginBlock) {
        ()
    }

    async fn end_block<S: StateWrite + 'static>(_state: &mut Arc<S>, _end_block: &EndBlock) {
        ()
    }

    // TODO: are we going to have epochs? might need to write out own Component trait
    async fn end_epoch<S: StateWrite + 'static>(_state: &mut Arc<S>) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transaction {
    to: String,
    from: String,
    amount: u64, // might need to be larger
    nonce: u32,
}

impl Transaction {
    pub fn new(to: String, from: String, amount: u64, nonce: u32) -> Self {
        Self {
            to,
            from,
            amount,
            nonce,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tx = serde_json::from_slice(bytes)?;
        Ok(tx)
    }

    pub fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    pub async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        let account_state = state
            .get_raw(&storage_key(&self.from))
            .await?
            .unwrap_or([0u8; 12].to_vec());
        let nonce = u32::from_be_bytes(account_state[0..4].try_into()?);

        // TODO: do nonces start at 0 or 1?
        if nonce <= self.nonce {
            anyhow::bail!("invalid nonce");
        }

        let balance = u64::from_be_bytes(account_state[4..12].try_into()?);
        if balance < self.amount {
            anyhow::bail!("insufficient funds");
        }

        Ok(())
    }

    pub async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        let from_balance = state
            .get_raw(&storage_key(&self.from))
            .await?
            .unwrap_or([0u8; 8].to_vec());
        let from_balance = u64::from_be_bytes(from_balance.as_slice().try_into()?);
        let to_balance = state
            .get_raw(&storage_key(&self.to))
            .await?
            .unwrap_or([0u8; 8].to_vec());
        let to_balance = u64::from_be_bytes(to_balance.as_slice().try_into()?);

        state.put_raw(
            storage_key(&self.from),
            (from_balance - self.amount).to_be_bytes().to_vec(),
        );
        state.put_raw(
            storage_key(&self.to),
            (to_balance + self.amount).to_be_bytes().to_vec(),
        );

        Ok(())
    }
}
