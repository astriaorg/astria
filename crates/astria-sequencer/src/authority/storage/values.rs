use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::{
    crypto::VerificationKey as DomainVerificationKey,
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1::action::{
        ValidatorName,
        ValidatorUpdate as DomainValidatorUpdate,
    },
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use crate::{
    accounts::AddressBytes as DomainAddressBytes,
    authority::ValidatorSet as DomainValidatorSet,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    AddressBytes(AddressBytes<'a>),
    ValidatorSet(ValidatorSet<'a>),
    ValidatorCount(ValidatorCount),
    ValidatorInfoV1(ValidatorInfoV1<'a>),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::authority) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

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
        crate::storage::StoredValue::Authority(Value(ValueImpl::AddressBytes(address)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for AddressBytes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Authority(Value(ValueImpl::AddressBytes(address))) = value
        else {
            bail!("authority stored value type mismatch: expected address bytes, found {value:?}");
        };
        Ok(address)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct VerificationKey<'a>(Cow<'a, [u8; 32]>);

impl Debug for VerificationKey<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a> From<VerificationKey<'a>> for DomainVerificationKey {
    fn from(value: VerificationKey<'a>) -> Self {
        DomainVerificationKey::try_from(value.0.into_owned())
            .expect("verification key in storage must be valid")
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct ValidatorUpdate<'a> {
    address_bytes: AddressBytes<'a>,
    power: u32,
    verification_key: VerificationKey<'a>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::authority) struct ValidatorSet<'a>(Vec<ValidatorUpdate<'a>>);

impl<'a> From<&'a DomainValidatorSet> for ValidatorSet<'a> {
    fn from(value: &'a DomainValidatorSet) -> Self {
        ValidatorSet(
            value
                .updates()
                .map(|update| ValidatorUpdate {
                    address_bytes: AddressBytes::from(&update.verification_key),
                    power: update.power,
                    verification_key: VerificationKey(Cow::Borrowed(
                        update.verification_key.as_bytes(),
                    )),
                })
                .collect(),
        )
    }
}

impl<'a> From<ValidatorSet<'a>> for DomainValidatorSet {
    fn from(value: ValidatorSet<'a>) -> Self {
        let inner = value
            .0
            .into_iter()
            .map(|update| {
                let key = <[u8; 20]>::from(update.address_bytes);
                let validator_update = DomainValidatorUpdate {
                    power: update.power,
                    verification_key: astria_core::crypto::VerificationKey::from(
                        update.verification_key,
                    ),
                    name: ValidatorName::empty(),
                };
                (key, validator_update)
            })
            .collect();
        DomainValidatorSet::new(inner)
    }
}

impl<'a> From<ValidatorSet<'a>> for crate::storage::StoredValue<'a> {
    fn from(validator_set: ValidatorSet<'a>) -> Self {
        crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorSet(validator_set)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ValidatorSet<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorSet(validator_set))) =
            value
        else {
            bail!("authority stored value type mismatch: expected validator set, found {value:?}");
        };
        Ok(validator_set)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::authority) struct ValidatorCount(u64);

impl From<u64> for ValidatorCount {
    fn from(value: u64) -> Self {
        ValidatorCount(value)
    }
}

impl From<ValidatorCount> for u64 {
    fn from(value: ValidatorCount) -> Self {
        value.0
    }
}

impl From<ValidatorCount> for crate::storage::StoredValue<'_> {
    fn from(validator_count: ValidatorCount) -> Self {
        crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorCount(validator_count)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ValidatorCount {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorCount(
            validator_count,
        ))) = value
        else {
            bail!(
                "authority stored value type mismatch: expected validator count, found {value:?}"
            );
        };
        Ok(validator_count)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::authority) struct ValidatorInfoV1<'a> {
    name: Cow<'a, str>,
    power: u32,
    verification_key: VerificationKey<'a>,
}

impl<'a> From<&'a DomainValidatorUpdate> for ValidatorInfoV1<'a> {
    fn from(value: &'a DomainValidatorUpdate) -> Self {
        ValidatorInfoV1 {
            name: Cow::Borrowed(value.name.as_str()),
            power: value.power,
            verification_key: VerificationKey(Cow::Borrowed(value.verification_key.as_bytes())),
        }
    }
}

impl From<ValidatorInfoV1<'_>> for DomainValidatorUpdate {
    fn from(value: ValidatorInfoV1) -> Self {
        Self {
            name: value
                .name
                .into_owned()
                .parse()
                .expect("state should only contain valid validator names"),
            power: value.power,
            verification_key: astria_core::crypto::VerificationKey::from(value.verification_key),
        }
    }
}

impl<'a> From<ValidatorInfoV1<'a>> for crate::storage::StoredValue<'a> {
    fn from(validator_info: ValidatorInfoV1<'a>) -> Self {
        crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorInfoV1(validator_info)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ValidatorInfoV1<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Authority(Value(ValueImpl::ValidatorInfoV1(
            validator_info,
        ))) = value
        else {
            bail!(
                "authority stored value type mismatch: expected validator info (v1), found \
                 {value:?}"
            );
        };
        Ok(validator_info)
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
            "value_impl_address_bytes",
            borsh_then_hex(&ValueImpl::AddressBytes((&[0; ADDRESS_LEN]).into()))
        );
        assert_snapshot!(
            "value_impl_validator_set",
            borsh_then_hex(&ValueImpl::ValidatorSet(ValidatorSet(vec![])))
        );
        assert_snapshot!(
            "value_impl_validator_count",
            borsh_then_hex(&ValueImpl::ValidatorCount(ValidatorCount(0)))
        );
        assert_snapshot!(
            "value_impl_validator_info_v1",
            borsh_then_hex(&ValueImpl::ValidatorInfoV1(ValidatorInfoV1 {
                name: Cow::Borrowed(""),
                power: 0,
                verification_key: VerificationKey(Cow::Borrowed(&[0; 32])),
            }))
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_authority_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_authority_variant",
            borsh_then_hex(&StoredValue::Authority(Value(ValueImpl::ValidatorSet(
                ValidatorSet(vec![])
            ))))
        );
    }
}
