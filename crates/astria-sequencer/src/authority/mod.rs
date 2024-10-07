mod action;
pub(crate) mod component;
mod state_ext;
pub(crate) mod storage;

use std::collections::BTreeMap;

use astria_core::{
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1alpha1::action::ValidatorUpdate,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

use crate::accounts::AddressBytes;

/// A map of public keys to validator updates.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ValidatorSet(BTreeMap<[u8; ADDRESS_LEN], ValidatorUpdate>);

impl ValidatorSet {
    pub(crate) fn new(inner: BTreeMap<[u8; ADDRESS_LEN], ValidatorUpdate>) -> Self {
        Self(inner)
    }

    pub(crate) fn new_from_updates(updates: Vec<ValidatorUpdate>) -> Self {
        Self(
            updates
                .into_iter()
                .map(|update| (*update.verification_key.address_bytes(), update))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    pub(crate) fn updates(&self) -> impl Iterator<Item = &ValidatorUpdate> {
        self.0.values()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn get<T: AddressBytes>(&self, address: &T) -> Option<&ValidatorUpdate> {
        self.0.get(address.address_bytes())
    }

    pub(super) fn push_update(&mut self, update: ValidatorUpdate) {
        self.0
            .insert(*update.verification_key.address_bytes(), update);
    }

    pub(super) fn remove<T: AddressBytes>(&mut self, address: &T) {
        self.0.remove(address.address_bytes());
    }

    /// Apply updates to the validator set.
    ///
    /// If the power of a validator is set to 0, remove it from the set.
    /// Otherwise, update the validator's power.
    pub(super) fn apply_updates(&mut self, validator_updates: ValidatorSet) {
        for (address, update) in validator_updates.0 {
            match update.power {
                0 => self.0.remove(&address),
                _ => self.0.insert(address, update),
            };
        }
    }

    pub(crate) fn try_into_cometbft(self) -> Result<Vec<tendermint::validator::Update>> {
        self.0
            .into_values()
            .map(crate::utils::sequencer_to_cometbft_validator)
            .collect::<Result<Vec<_>, _>>()
            .wrap_err("failed to map one or more astria validators to cometbft validators")
    }
}
