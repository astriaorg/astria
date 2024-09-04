mod action;
pub(crate) mod component;
mod state_ext;

use std::collections::BTreeMap;

use anyhow::Context as _;
use astria_core::{
    crypto::VerificationKey,
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1alpha1::action::ValidatorUpdate,
};
use serde::{
    Deserialize,
    Serialize,
};
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct ValidatorSetKey(#[serde(with = "::hex::serde")] [u8; ADDRESS_LEN]);

impl From<[u8; ADDRESS_LEN]> for ValidatorSetKey {
    fn from(value: [u8; ADDRESS_LEN]) -> Self {
        Self(value)
    }
}

impl From<&VerificationKey> for ValidatorSetKey {
    fn from(value: &VerificationKey) -> Self {
        Self(value.address_bytes())
    }
}

/// Newtype wrapper to read and write a validator set or set of updates from rocksdb.
///
/// Contains a map of hex-encoded public keys to validator updates.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ValidatorSet(BTreeMap<ValidatorSetKey, ValidatorUpdate>);

impl ValidatorSet {
    pub(crate) fn new_from_updates(updates: Vec<ValidatorUpdate>) -> Self {
        Self(
            updates
                .into_iter()
                .map(|update| ((&update.verification_key).into(), update))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn get<T: Into<ValidatorSetKey>>(&self, address: T) -> Option<&ValidatorUpdate> {
        self.0.get(&address.into())
    }

    pub(super) fn push_update(&mut self, update: ValidatorUpdate) {
        self.0.insert((&update.verification_key).into(), update);
    }

    pub(super) fn remove<T: Into<ValidatorSetKey>>(&mut self, address: T) {
        self.0.remove(&address.into());
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

    pub(crate) fn try_into_cometbft(self) -> anyhow::Result<Vec<tendermint::validator::Update>> {
        self.0
            .into_values()
            .map(crate::utils::sequencer_to_cometbft_validator)
            .collect::<Result<Vec<_>, _>>()
            .context("failed to map one or more astria validators to cometbft validators")
    }
}
