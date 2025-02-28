use std::borrow::Cow;

use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    AddressPrefix(AddressPrefix<'a>),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::address) struct AddressPrefix<'a>(Cow<'a, str>);

impl<'a> From<&'a str> for AddressPrefix<'a> {
    fn from(prefix: &'a str) -> Self {
        AddressPrefix(Cow::Borrowed(prefix))
    }
}

impl<'a> From<AddressPrefix<'a>> for String {
    fn from(prefix: AddressPrefix<'a>) -> Self {
        prefix.0.into_owned()
    }
}

impl<'a> From<AddressPrefix<'a>> for crate::storage::StoredValue<'a> {
    fn from(prefix: AddressPrefix<'a>) -> Self {
        crate::storage::StoredValue::Address(Value(ValueImpl::AddressPrefix(prefix)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressPrefix<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Address(Value(ValueImpl::AddressPrefix(prefix))) = value
        else {
            bail!("address stored value type mismatch: expected address prefix, found {value:?}");
        };
        Ok(prefix)
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
            "value_impl_address_prefix",
            borsh_then_hex(&ValueImpl::AddressPrefix("test_prefix".into()))
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_address_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_address_variant",
            borsh_then_hex(&StoredValue::Address(Value(ValueImpl::AddressPrefix(
                "test_prefix".into()
            ))))
        );
    }
}
