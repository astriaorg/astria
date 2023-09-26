use std::collections::HashMap;

use anyhow::{
    anyhow,
    bail,
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

/// Newtype wrapper to read and write a validator set or set of updates from rocksdb.
///
/// Contains a map of hex-encoded public keys to validator updates.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ValidatorSet(HashMap<String, tendermint::validator::Update>);

impl ValidatorSet {
    pub(crate) fn new_from_updates(updates: Vec<tendermint::validator::Update>) -> Self {
        let validator_set = updates
            .into_iter()
            .map(|update| (update.pub_key.to_hex(), update))
            .collect();
        Self(validator_set)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub(crate) fn get(
        &self,
        pub_key: &tendermint::public_key::PublicKey,
    ) -> Option<&tendermint::validator::Update> {
        self.0.get(&pub_key.to_hex())
    }

    pub(crate) fn push_update(&mut self, update: tendermint::validator::Update) {
        self.0.insert(update.pub_key.to_hex(), update);
    }

    /// Apply updates to the validator set.
    ///
    /// If the power of a validator is set to 0, remove it from the set.
    /// Otherwise, update the validator's power.
    pub(crate) fn apply_updates(&mut self, validator_updates: ValidatorSet) {
        for (pub_key, update) in validator_updates.0 {
            match update.power.value() {
                0 => self.0.remove(&pub_key),
                _ => self.0.insert(pub_key, update),
            };
        }
    }

    pub(crate) fn into_tendermint_validator_updates(self) -> Vec<tendermint::validator::Update> {
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
            bail!("validator set not found")
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
            return Ok(ValidatorSet(HashMap::new()));
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
            SudoAddress(address.0)
                .try_to_vec()
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
        self.nonconsensus_put_raw(
            VALIDATOR_UPDATES_KEY.to_vec(),
            serde_json::to_vec(&validator_updates)
                .context("failed to serialize validator updates")?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn clear_validator_updates(&mut self) {
        self.nonconsensus_delete(VALIDATOR_UPDATES_KEY.to_vec());
    }
}

impl<T: StateWrite> StateWriteExt for T {}
