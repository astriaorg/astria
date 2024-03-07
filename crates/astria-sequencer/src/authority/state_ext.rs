use std::collections::BTreeMap;

use anyhow::{
    bail,
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    Address,
    ADDRESS_LEN,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::{
    account,
    validator,
};
use tracing::instrument;

/// Newtype wrapper to read and write an address from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoAddress([u8; ADDRESS_LEN]);

/// Newtype wrapper to read and write a validator set or set of updates from rocksdb.
///
/// Contains a map of hex-encoded public keys to validator updates.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ValidatorSet(BTreeMap<account::Id, validator::Update>);

impl ValidatorSet {
    pub(crate) fn new_from_updates(updates: Vec<validator::Update>) -> Self {
        let validator_set = updates
            .into_iter()
            .map(|update| (account::Id::from(update.pub_key), update))
            .collect::<BTreeMap<_, _>>();
        Self(validator_set)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub(crate) fn get(&self, address: &account::Id) -> Option<&validator::Update> {
        self.0.get(address)
    }

    pub(crate) fn push_update(&mut self, update: validator::Update) {
        let address = tendermint::account::Id::from(update.pub_key);
        self.0.insert(address, update);
    }

    pub(crate) fn remove(&mut self, address: &account::Id) {
        self.0.remove(address);
    }

    /// Apply updates to the validator set.
    ///
    /// If the power of a validator is set to 0, remove it from the set.
    /// Otherwise, update the validator's power.
    pub(crate) fn apply_updates(&mut self, validator_updates: ValidatorSet) {
        for (address, update) in validator_updates.0 {
            match update.power.value() {
                0 => self.0.remove(&address),
                _ => self.0.insert(address, update),
            };
        }
    }

    pub(crate) fn into_tendermint_validator_updates(self) -> Vec<validator::Update> {
        self.0.into_values().collect::<Vec<_>>()
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
            bail!("sudo key not found");
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
            bail!("validator set not found")
        };

        let ValidatorSet(validator_set) =
            serde_json::from_slice(&bytes).context("invalid validator set bytes")?;
        Ok(ValidatorSet(validator_set))
    }

    #[instrument(skip(self))]
    async fn get_validator_updates(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .nonverifiable_get_raw(VALIDATOR_UPDATES_KEY)
            .await
            .context("failed reading raw validator updates from state")?
        else {
            // return empty set because validator updates are optional
            return Ok(ValidatorSet(BTreeMap::new()));
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
            borsh::to_vec(&SudoAddress(address.0))
                .context("failed to convert sudo address to vec")?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_validator_set(&mut self, validator_set: ValidatorSet) -> Result<()> {
        self.put_raw(
            VALIDATOR_SET_STORAGE_KEY.to_string(),
            serde_json::to_vec(&validator_set).context("failed to serialize validator set")?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_validator_updates(&mut self, validator_updates: ValidatorSet) -> Result<()> {
        self.nonverifiable_put_raw(
            VALIDATOR_UPDATES_KEY.to_vec(),
            serde_json::to_vec(&validator_updates)
                .context("failed to serialize validator updates")?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn clear_validator_updates(&mut self) {
        self.nonverifiable_delete(VALIDATOR_UPDATES_KEY.to_vec());
    }
}

impl<T: StateWrite> StateWriteExt for T {}
