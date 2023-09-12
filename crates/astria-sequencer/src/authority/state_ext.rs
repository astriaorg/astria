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
use tracing::instrument;

/// Newtype wrapper to read and write an address from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoAddress([u8; ADDRESS_LEN]);

/// Newtype wrapper to read and write a validator set from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub(crate) struct ValidatorSet(pub(crate) Vec<Validator>);

impl TryFrom<Vec<tendermint::validator::Update>> for ValidatorSet {
    type Error = anyhow::Error;

    fn try_from(updates: Vec<tendermint::validator::Update>) -> Result<Self> {
        Ok(ValidatorSet(
            updates
                .into_iter()
                .map(|update| {
                    let public_key = update
                        .pub_key
                        .to_bytes()
                        .try_into()
                        .map_err(|_| anyhow!("public key must be 32 bytes"))?;
                    Ok(Validator {
                        public_key,
                        voting_power: update.power.into(),
                    })
                })
                .collect::<Result<Vec<Validator>>>()?,
        ))
    }
}

/// Newtype wrapper to read and write a validator update from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub(crate) struct ValidatorUpdates(pub(crate) Vec<Validator>);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub(crate) struct Validator {
    pub(crate) public_key: [u8; 32],
    pub(crate) voting_power: u64,
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
            return Err(anyhow!("validator set not found"));
        };

        let ValidatorSet(validator_set) =
            ValidatorSet::try_from_slice(&bytes).context("invalid validator set bytes")?;
        Ok(ValidatorSet(validator_set))
    }

    #[instrument(skip(self))]
    async fn get_validator_updates(&self) -> Result<ValidatorUpdates> {
        let Some(bytes) = self
            .nonconsensus_get_raw(VALIDATOR_UPDATES_KEY)
            .await
            .context("failed reading raw validator updates from state")?
        else {
            return Err(anyhow!("validator updates not found"));
        };

        let validator_updates: ValidatorUpdates =
            ValidatorUpdates::try_from_slice(&bytes).context("invalid validator updates bytes")?;
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
            validator_set.try_to_vec()?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_validator_updates(&mut self, validator_updates: ValidatorUpdates) -> Result<()> {
        self.nonconsensus_put_raw(
            VALIDATOR_UPDATES_KEY.to_vec(),
            validator_updates.try_to_vec()?,
        );
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
