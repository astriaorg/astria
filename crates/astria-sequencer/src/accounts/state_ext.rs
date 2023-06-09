use anyhow::{
    ensure,
    Context,
    Result,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

pub type Address = String;
pub type Balance = u64; // might need to be larger
pub type Nonce = u32;

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
    async fn get_account_balance(&self, address: &str) -> Result<Balance> {
        let bytes = self
            .get_raw(&balance_storage_key(address))
            .await
            .context("storage error")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(0);
        };

        ensure!(
            bytes.len() == 8,
            "invalid balance length: expected 8, got {}",
            bytes.len()
        );

        let balance = u64::from_be_bytes(bytes[0..8].try_into()?);
        Ok(balance)
    }

    #[instrument(skip(self))]
    async fn get_account_nonce(&self, address: &str) -> Result<Nonce> {
        let bytes = self
            .get_raw(&nonce_storage_key(address))
            .await
            .context("storage error")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return (0, 0)
            return Ok(0);
        };

        ensure!(
            bytes.len() == 4,
            "invalid nonce length: expected 4, got {}",
            bytes.len()
        );

        let nonce = u32::from_be_bytes(bytes[0..4].try_into()?);
        Ok(nonce)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub trait StateWriteExt: StateWrite {
    fn put_account_balance(&mut self, address: &str, balance: Balance) {
        let bytes = balance.to_be_bytes().to_vec();
        self.put_raw(balance_storage_key(address), bytes);
    }

    fn put_account_nonce(&mut self, address: &str, nonce: Nonce) {
        let bytes = nonce.to_be_bytes().to_vec();
        self.put_raw(nonce_storage_key(address), bytes);
    }
}

impl<T: StateWrite> StateWriteExt for T {}
