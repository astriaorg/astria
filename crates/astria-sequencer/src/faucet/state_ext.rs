use anyhow::{
    Context,
    Result,
};
use astria_proto::native::sequencer::v1alpha1::Address;
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use hex::ToHex as _;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tracing::{
    debug,
    instrument,
};

use crate::accounts::types::Balance;

const FAUCET_PREFIX: &str = "faucet";

fn storage_key(address: &Address) -> String {
    let address = address.encode_hex::<String>();
    format!("{FAUCET_PREFIX}/{address}")
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub(crate) struct AccountInfo {
    // the amount of funds that can be requested from the faucet until `reset_time`
    pub(crate) amount_remaining: Balance,
    // unix timestamp where the amount remaining will be reset
    pub(crate) reset_time: u64,
}

impl Default for AccountInfo {
    fn default() -> Self {
        Self {
            amount_remaining: crate::faucet::action::FAUCET_LIMIT_PER_DAY,
            reset_time: 0,
        }
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_account_info(&self, address: Address) -> Result<AccountInfo> {
        let Some(bytes) = self
            .get_raw(&storage_key(&address))
            .await
            .context("failed reading raw account info from state")?
        else {
            debug!("account info not found, returning default");
            return Ok(AccountInfo::default());
        };
        let info = AccountInfo::try_from_slice(&bytes).context("invalid account info bytes")?;
        Ok(info)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_account_info(&mut self, address: Address, info: AccountInfo) -> Result<()> {
        let bytes = info.try_to_vec().context("failed to serialize balance")?;
        self.put_raw(storage_key(&address), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
