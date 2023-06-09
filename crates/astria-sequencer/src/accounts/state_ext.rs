use anyhow::{
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize as _,
    BorshSerialize as _,
};
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use crate::accounts::types::{
    Address,
    Balance,
    Nonce,
};

const ACCOUNTS_PREFIX: &str = "accounts";

fn storage_key(address: &str) -> String {
    format!("{}/{}", ACCOUNTS_PREFIX, address)
}

pub(crate) fn balance_storage_key(address: &str) -> String {
    format!("{}/balance", storage_key(address))
}

pub(crate) fn nonce_storage_key(address: &str) -> String {
    format!("{}/nonce", storage_key(address))
}

#[async_trait]
pub trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_account_balance(&self, address: &Address) -> Result<Balance> {
        let bytes = self
            .get_raw(&balance_storage_key(address.to_str()))
            .await
            .context("storage error")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(Balance::from(0));
        };

        let balance = Balance::try_from_slice(&bytes).context("invalid balance bytes")?;
        Ok(balance)
    }

    #[instrument(skip(self))]
    async fn get_account_nonce(&self, address: &Address) -> Result<Nonce> {
        let bytes = self
            .get_raw(&nonce_storage_key(address.to_str()))
            .await
            .context("storage error")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(Nonce::from(0));
        };

        let nonce = Nonce::try_from_slice(&bytes).context("invalid nonce bytes")?;
        Ok(nonce)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_account_balance(&mut self, address: &Address, balance: Balance) -> Result<()> {
        let bytes = balance
            .try_to_vec()
            .context("failed to serialize balance")?;
        self.put_raw(balance_storage_key(address.to_str()), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_account_nonce(&mut self, address: &Address, nonce: Nonce) -> Result<()> {
        let bytes = nonce.try_to_vec().context("failed to serialize nonce")?;
        self.put_raw(nonce_storage_key(address.to_str()), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
