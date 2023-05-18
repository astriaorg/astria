use anyhow::{
    Result,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};

use crate::accounts::{transaction::{Balance, Nonce}};

const ACCOUNTS_PREFIX: &str = "accounts";

fn storage_key(address: &str) -> String {
    format!("{}/{}", ACCOUNTS_PREFIX, address)
}

#[async_trait]
pub trait StateReadExt: StateRead {
    async fn get_account_state(&self, address: &str) -> Result<(Balance, Nonce)> {
        let bytes = self.get_raw(&storage_key(address)).await?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return (0, 0)
            return Ok((0, 0));
        };

        let balance = u64::from_be_bytes(bytes[0..8].try_into()?);
        let nonce = u32::from_be_bytes(bytes[8..12].try_into()?);
        Ok((balance, nonce))
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub trait StateWriteExt: StateWrite {
    fn put_account_state(&mut self, address: &str, balance: Balance, nonce: Nonce) {
        let mut bytes = balance.to_be_bytes().to_vec();
        bytes.append(&mut nonce.to_be_bytes().to_vec());
        self.put_raw(storage_key(address), bytes);
    }
}

impl<T: StateWrite> StateWriteExt for T {}
