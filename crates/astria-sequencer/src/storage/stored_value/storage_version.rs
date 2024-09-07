use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct StorageVersion(u64);

impl From<u64> for StorageVersion {
    fn from(storage_version: u64) -> Self {
        StorageVersion(storage_version)
    }
}

impl From<StorageVersion> for u64 {
    fn from(storage_version: StorageVersion) -> Self {
        storage_version.0
    }
}

impl<'a> TryFrom<StoredValue<'a>> for StorageVersion {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::StorageVersion(storage_version) = value else {
            return Err(super::type_mismatch("storage version", &value));
        };
        Ok(storage_version)
    }
}
