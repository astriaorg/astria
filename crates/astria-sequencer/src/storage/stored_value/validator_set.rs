use std::borrow::Cow;

use astria_core::{
    crypto::VerificationKey as DomainVerificationKey,
    protocol::transaction::v1alpha1::action::ValidatorUpdate as DomainValidatorUpdate,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    AddressBytes,
    StoredValue,
};
use crate::authority::ValidatorSet as DomainValidatorSet;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct VerificationKey<'a>(Cow<'a, [u8; 32]>);

impl<'a> From<VerificationKey<'a>> for DomainVerificationKey {
    fn from(value: VerificationKey<'a>) -> Self {
        DomainVerificationKey::try_from(value.0.into_owned())
            .expect("verification key in storage must be valid")
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct ValidatorUpdate<'a> {
    address_bytes: AddressBytes<'a>,
    power: u32,
    verification_key: VerificationKey<'a>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct ValidatorSet<'a>(Vec<ValidatorUpdate<'a>>);

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

impl<'a> TryFrom<StoredValue<'a>> for ValidatorSet<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::ValidatorSet(validator_set) = value else {
            return Err(super::type_mismatch("validator set", &value));
        };
        Ok(validator_set)
    }
}
