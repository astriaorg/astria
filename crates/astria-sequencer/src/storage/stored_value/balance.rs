use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Balance(u128);

impl From<u128> for Balance {
    fn from(balance: u128) -> Self {
        Balance(balance)
    }
}

impl From<Balance> for u128 {
    fn from(balance: Balance) -> Self {
        balance.0
    }
}

impl<'a> TryFrom<StoredValue<'a>> for Balance {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::Balance(balance) = value else {
            return Err(super::type_mismatch("balance", &value));
        };
        Ok(balance)
    }
}
