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

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value(ValueImpl);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl {
    Balance(Balance),
    Nonce(Nonce),
    Fee(Fee),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ValueImpl::Balance(balance) => write!(f, "balance {}", balance.0),
            ValueImpl::Nonce(nonce) => write!(f, "nonce {}", nonce.0),
            ValueImpl::Fee(fee) => write!(f, "fee {}", fee.0),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::accounts) struct Balance(u128);

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

impl<'a> From<Balance> for crate::storage::StoredValue<'a> {
    fn from(balance: Balance) -> Self {
        crate::storage::StoredValue::Accounts(Value(ValueImpl::Balance(balance)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Balance {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Accounts(Value(ValueImpl::Balance(balance))) = value
        else {
            bail!("accounts stored value type mismatch: expected balance, found {value}");
        };
        Ok(balance)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::accounts) struct Nonce(u32);

impl From<u32> for Nonce {
    fn from(nonce: u32) -> Self {
        Nonce(nonce)
    }
}

impl From<Nonce> for u32 {
    fn from(nonce: Nonce) -> Self {
        nonce.0
    }
}

impl<'a> From<Nonce> for crate::storage::StoredValue<'a> {
    fn from(nonce: Nonce) -> Self {
        crate::storage::StoredValue::Accounts(Value(ValueImpl::Nonce(nonce)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Nonce {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Accounts(Value(ValueImpl::Nonce(nonce))) = value else {
            bail!("accounts stored value type mismatch: expected nonce, found {value}");
        };
        Ok(nonce)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::accounts) struct Fee(u128);

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

impl<'a> From<Fee> for crate::storage::StoredValue<'a> {
    fn from(fee: Fee) -> Self {
        crate::storage::StoredValue::Accounts(Value(ValueImpl::Fee(fee)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Fee {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Accounts(Value(ValueImpl::Fee(fee))) = value else {
            bail!("accounts stored value type mismatch: expected fee, found {value}");
        };
        Ok(fee)
    }
}
