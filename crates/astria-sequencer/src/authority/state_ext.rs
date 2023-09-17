use anyhow::{
    anyhow,
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use proto::native::sequencer::v1alpha1::{
    Address,
    ADDRESS_LEN,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

/// Newtype wrapper to read and write an address from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoAddress([u8; ADDRESS_LEN]);

/// Newtype wrapper to read and write a validator set from rocksdb.
///
/// Note: this is stored only in the nonconsensus state, thus is not part
/// of the application state, so it's fine for it to be serialized/deserialized
/// in a non-deterministic way (ie. with serde-json).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ValidatorSet(pub(crate) Vec<tendermint::validator::Update>);

impl ValidatorSet {
    pub(crate) fn apply_updates(&mut self, validator_updates: &ValidatorSet) {
        for curr in &mut self.0 {
            for update in &validator_updates.0 {
                if curr.pub_key == update.pub_key {
                    curr.power = update.power;
                }
            }
        }
    }
}

const SUDO_STORAGE_KEY: &str = "sudo";
const VALIDATOR_SET_STORAGE_KEY: &str = "valset";
const VALIDATOR_UPDATES_KEY: &[u8] = b"valupdates";

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_sudo_address(&self) -> Result<Address> {
        let Some(bytes) = self
            .get_raw(SUDO_STORAGE_KEY)
            .await
            .context("failed reading raw sudo key from state")?
        else {
            // return error because sudo key must be set
            return Err(anyhow!("sudo key not found"));
        };
        let SudoAddress(address) =
            SudoAddress::try_from_slice(&bytes).context("invalid sudo key bytes")?;
        Ok(Address(address))
    }

    #[instrument(skip(self))]
    async fn get_validator_set(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .get_raw(VALIDATOR_SET_STORAGE_KEY)
            .await
            .context("failed reading raw validator set from state")?
        else {
            // return error because validator set must be set
            return Err(anyhow!("validator set not found"));
        };

        let ValidatorSet(validator_set) =
            serde_json::from_slice(&bytes).context("invalid validator set bytes")?;
        Ok(ValidatorSet(validator_set))
    }

    #[instrument(skip(self))]
    async fn get_validator_updates(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .nonconsensus_get_raw(VALIDATOR_UPDATES_KEY)
            .await
            .context("failed reading raw validator updates from state")?
        else {
            // return empty set because validator updates are optional
            return Ok(ValidatorSet(vec![]));
        };

        let validator_updates: ValidatorSet =
            serde_json::from_slice(&bytes).context("invalid validator updates bytes")?;
        Ok(validator_updates)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_sudo_address(&mut self, address: Address) -> Result<()> {
        self.put_raw(
            SUDO_STORAGE_KEY.to_string(),
            SudoAddress(address.0).try_to_vec()?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_validator_set(&mut self, validator_set: ValidatorSet) -> Result<()> {
        self.put_raw(
            VALIDATOR_SET_STORAGE_KEY.to_string(),
            serde_json::to_vec(&validator_set)?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_validator_updates(&mut self, validator_updates: ValidatorSet) -> Result<()> {
        self.nonconsensus_put_raw(
            VALIDATOR_UPDATES_KEY.to_vec(),
            serde_json::to_vec(&validator_updates)?,
        );
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
