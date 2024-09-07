use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct RevisionNumber(u64);

impl From<u64> for RevisionNumber {
    fn from(revision_number: u64) -> Self {
        RevisionNumber(revision_number)
    }
}

impl From<RevisionNumber> for u64 {
    fn from(revision_number: RevisionNumber) -> Self {
        revision_number.0
    }
}

impl<'a> TryFrom<StoredValue<'a>> for RevisionNumber {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::RevisionNumber(revision_number) = value else {
            return Err(super::type_mismatch("revision number", &value));
        };
        Ok(revision_number)
    }
}
