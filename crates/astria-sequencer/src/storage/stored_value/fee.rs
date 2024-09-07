use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Fee(u128);

impl From<u128> for Fee {
    fn from(fee: u128) -> Self {
        Fee(fee)
    }
}

impl From<Fee> for u128 {
    fn from(fee: Fee) -> Self {
        fee.0
    }
}

impl<'a> TryFrom<StoredValue<'a>> for Fee {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::Fee(fee) = value else {
            return Err(super::type_mismatch("fee", &value));
        };
        Ok(fee)
    }
}
