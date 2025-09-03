use std::fmt::{
    self,
    Display,
    Formatter,
};

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
pub(in crate::bridge) struct DisabledStatus(bool);

impl Display for DisabledStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<bool> for DisabledStatus {
    fn from(disabled: bool) -> Self {
        DisabledStatus(disabled)
    }
}

impl From<DisabledStatus> for bool {
    fn from(disabled: DisabledStatus) -> Self {
        disabled.0
    }
}

impl From<DisabledStatus> for crate::storage::StoredValue<'_> {
    fn from(disabled: DisabledStatus) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::DisabledStatus(disabled)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for DisabledStatus {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::DisabledStatus(disabled))) = value
        else {
            bail!("bridge stored value type mismatch: expected disabled status, found {value:?}");
        };
        Ok(disabled)
    }
}
