use anyhow::{
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use ed25519_consensus::VerificationKey;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

/// Newtype wrapper to read and write a 32-byte array from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoPublicKey([u8; 32]);

/// Newtype wrapper to read and write a validator set from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct ValidatorSet(Vec<Validator>);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Validator {
    public_key: [u8; 32],
    voting_power: u64, // set to 0 to remove validator
}

const SUDO_STORAGE_KEY: &str = "sudo";
const VALIDATOR_SET_STORAGE_KEY: &str = "valset";

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_sudo_key(&self) -> Result<Option<VerificationKey>> {
        let Some(bytes) = self
            .get_raw(SUDO_STORAGE_KEY)
            .await
            .context("failed reading raw sudo key from state")?
        else {
            // TODO: should this panic?
            return Ok(None);
        };
        let SudoPublicKey(key) =
            SudoPublicKey::try_from_slice(&bytes).context("invalid sudo key bytes")?;
        Ok(Some(
            VerificationKey::try_from(key).context("invalid sudo key")?,
        ))
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_sudo_key(&mut self, key: VerificationKey) -> Result<()> {
        let key = SudoPublicKey(key.to_bytes());
        self.put_raw(SUDO_STORAGE_KEY.to_string(), key.try_to_vec()?);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
