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

impl From<Balance> for crate::storage::StoredValue<'_> {
    fn from(balance: Balance) -> Self {
        crate::storage::StoredValue::Accounts(Value(ValueImpl::Balance(balance)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Balance {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Accounts(Value(ValueImpl::Balance(balance))) = value
        else {
            bail!("accounts stored value type mismatch: expected balance, found {value:?}");
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

impl From<Nonce> for crate::storage::StoredValue<'_> {
    fn from(nonce: Nonce) -> Self {
        crate::storage::StoredValue::Accounts(Value(ValueImpl::Nonce(nonce)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Nonce {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Accounts(Value(ValueImpl::Nonce(nonce))) = value else {
            bail!("accounts stored value type mismatch: expected nonce, found {value:?}");
        };
        Ok(nonce)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::test_utils::borsh_then_hex;

    #[test]
    fn value_impl_existing_variants_unchanged() {
        assert_snapshot!(
            "value_impl_balance",
            borsh_then_hex(&ValueImpl::Balance(Balance(0)))
        );
        assert_snapshot!(
            "value_impl_nonce",
            borsh_then_hex(&ValueImpl::Nonce(Nonce(0)))
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_account_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_account_variant",
            borsh_then_hex(&StoredValue::Accounts(Value(ValueImpl::Nonce(Nonce(0)))))
        );
    }
}
