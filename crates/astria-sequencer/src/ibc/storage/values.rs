use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::primitive::v1::ADDRESS_LEN;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use crate::accounts::AddressBytes as DomainAddressBytes;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    Balance(Balance),
    AddressBytes(AddressBytes<'a>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::ibc) struct Balance(u128);

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
        crate::storage::StoredValue::Ibc(Value(ValueImpl::Balance(balance)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Balance {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value(ValueImpl::Balance(balance))) = value else {
            bail!("ibc stored value type mismatch: expected balance, found {value:?}");
        };
        Ok(balance)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::ibc) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

impl Debug for AddressBytes<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a, T: DomainAddressBytes> From<&'a T> for AddressBytes<'a> {
    fn from(value: &'a T) -> Self {
        AddressBytes(Cow::Borrowed(value.address_bytes()))
    }
}

impl<'a> From<AddressBytes<'a>> for [u8; ADDRESS_LEN] {
    fn from(address_bytes: AddressBytes<'a>) -> Self {
        address_bytes.0.into_owned()
    }
}

impl<'a> From<AddressBytes<'a>> for crate::storage::StoredValue<'a> {
    fn from(address: AddressBytes<'a>) -> Self {
        crate::storage::StoredValue::Ibc(Value(ValueImpl::AddressBytes(address)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressBytes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Ibc(Value(ValueImpl::AddressBytes(address))) = value
        else {
            bail!("ibc stored value type mismatch: expected address bytes, found {value:?}");
        };
        Ok(address)
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
            "value_impl_address_bytes",
            borsh_then_hex(&ValueImpl::AddressBytes((&[0; ADDRESS_LEN]).into()))
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_ibc_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_ibc_variant",
            borsh_then_hex(&StoredValue::Ibc(Value(ValueImpl::Balance(Balance(0)))))
        );
    }
}
