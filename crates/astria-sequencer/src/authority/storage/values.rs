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
    protocol::transaction::v1alpha1::action::ValidatorUpdate as DomainValidatorUpdate,
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
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::authority) struct AddressBytes<'a>(Cow<'a, [u8; ADDRESS_LEN]>);

impl<'a> Debug for AddressBytes<'a> {
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

impl<'a> Debug for VerificationKey<'a> {
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
