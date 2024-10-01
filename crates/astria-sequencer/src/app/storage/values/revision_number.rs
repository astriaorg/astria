use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::app) struct RevisionNumber(u64);

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

impl<'a> From<RevisionNumber> for crate::storage::StoredValue<'a> {
    fn from(revision_number: RevisionNumber) -> Self {
        crate::storage::StoredValue::App(Value(ValueImpl::RevisionNumber(revision_number)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for RevisionNumber {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::App(Value(ValueImpl::RevisionNumber(revision_number))) =
            value
        else {
            bail!("app stored value type mismatch: expected revision number, found {value:?}");
        };
        Ok(revision_number)
    }
}
