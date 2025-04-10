use std::sync::Arc;

use borsh::BorshSerialize;

use super::{
    Change,
    ChangeName,
    UpgradeName,
};
use crate::{
    generated::upgrades::v1::{
        aspen::{
            IbcAcknowledgementFailureChange as RawIbcAcknowledgementFailureChange,
            PriceFeedChange as RawPriceFeedChange,
            ValidatorUpdateActionChange as RawValidatorUpdateActionChange,
        },
        Aspen as RawAspen,
        BaseUpgradeInfo as RawBaseUpgradeInfo,
    },
    protocol::genesis::v1::{
        PriceFeedGenesis,
        PriceFeedGenesisError,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
pub struct Aspen {
    activation_height: u64,
    app_version: u64,
    price_feed_change: PriceFeedChange,
    validator_update_action_change: ValidatorUpdateActionChange,
    ibc_acknowledgement_failure_change: IbcAcknowledgementFailureChange,
}

impl Aspen {
    pub const NAME: UpgradeName = UpgradeName::new("aspen");

    #[must_use]
    pub fn activation_height(&self) -> u64 {
        self.activation_height
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        self.app_version
    }

    #[must_use]
    pub fn price_feed_change(&self) -> &PriceFeedChange {
        &self.price_feed_change
    }

    #[must_use]
    pub fn validator_update_action_change(&self) -> &ValidatorUpdateActionChange {
        &self.validator_update_action_change
    }

    #[must_use]
    pub fn ibc_acknowledgement_failure_change(&self) -> &IbcAcknowledgementFailureChange {
        &self.ibc_acknowledgement_failure_change
    }

    pub fn changes(&self) -> impl Iterator<Item = &'_ dyn Change> {
        Some(&self.price_feed_change as &dyn Change)
            .into_iter()
            .chain(Some(&self.validator_update_action_change as &dyn Change))
            .chain(Some(
                &self.ibc_acknowledgement_failure_change as &dyn Change,
            ))
    }
}

impl Protobuf for Aspen {
    type Error = Error;
    type Raw = RawAspen;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let RawBaseUpgradeInfo {
            activation_height,
            app_version,
        } = *raw.base_info.as_ref().ok_or_else(Error::no_base_info)?;

        let price_feed_change = raw
            .price_feed_change
            .as_ref()
            .ok_or_else(Error::no_price_feed_change)?;

        let genesis = price_feed_change
            .genesis
            .as_ref()
            .ok_or_else(Error::no_price_feed_genesis)
            .and_then(|raw_genesis| {
                PriceFeedGenesis::try_from_raw_ref(raw_genesis).map_err(Error::price_feed_genesis)
            })?;

        if raw.validator_update_action_change.is_none() {
            return Err(Error::no_validator_update_action_change());
        }

        if raw.ibc_acknowledgement_failure_change.is_none() {
            return Err(Error::ibc_acknowledgement_failure_change());
        }

        let price_feed_change = PriceFeedChange {
            activation_height,
            app_version,
            genesis: Arc::new(genesis),
        };

        let validator_update_action_change = ValidatorUpdateActionChange {
            activation_height,
            app_version,
        };

        let ibc_acknowledgement_failure_change = IbcAcknowledgementFailureChange {
            activation_height,
            app_version,
        };

        Ok(Self {
            activation_height,
            app_version,
            price_feed_change,
            validator_update_action_change,
            ibc_acknowledgement_failure_change,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let base_info = Some(RawBaseUpgradeInfo {
            activation_height: self.activation_height,
            app_version: self.app_version,
        });
        let price_feed_change = Some(RawPriceFeedChange {
            genesis: Some(self.price_feed_change.genesis.to_raw()),
        });
        RawAspen {
            base_info,
            price_feed_change,
            validator_update_action_change: Some(RawValidatorUpdateActionChange {}),
            ibc_acknowledgement_failure_change: Some(RawIbcAcknowledgementFailureChange {}),
        }
    }
}

/// This change enables vote extensions and starts to provide price feed data from the price feed
///  (if enabled) via the vote extensions.
///
/// The vote extensions are enabled in the block immediately after `activation_height`, meaning the
/// price feed data is available no earlier than two blocks after `activation_height`.
#[derive(Clone, Debug, BorshSerialize)]
pub struct PriceFeedChange {
    activation_height: u64,
    app_version: u64,
    genesis: Arc<PriceFeedGenesis>,
}

impl PriceFeedChange {
    pub const NAME: ChangeName = ChangeName::new("price_feed_change");

    #[must_use]
    pub fn genesis(&self) -> &Arc<PriceFeedGenesis> {
        &self.genesis
    }
}

impl Change for PriceFeedChange {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// This change introduces new sequencer `Action`s to support updating the validator set.
#[derive(Clone, Debug, BorshSerialize)]
pub struct ValidatorUpdateActionChange {
    activation_height: u64,
    app_version: u64,
}

impl ValidatorUpdateActionChange {
    pub const NAME: ChangeName = ChangeName::new("validator_update_action_change");
}

impl Change for ValidatorUpdateActionChange {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// This change causes a fixed string to be used as the error message in an ICS20 transfer failure
/// acknowledgement.
#[derive(Clone, Debug, BorshSerialize)]
pub struct IbcAcknowledgementFailureChange {
    activation_height: u64,
    app_version: u64,
}

impl IbcAcknowledgementFailureChange {
    pub const NAME: ChangeName = ChangeName::new("ibc_acknowledgement_failure_change");
}

impl Change for IbcAcknowledgementFailureChange {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// An error when transforming a [`RawPriceFeedUpgrade`] into a [`PriceFeedChange`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn no_base_info() -> Self {
        Self(ErrorKind::FieldNotSet("base_info"))
    }

    fn no_price_feed_change() -> Self {
        Self(ErrorKind::FieldNotSet("price_feed_change"))
    }

    fn no_validator_update_action_change() -> Self {
        Self(ErrorKind::FieldNotSet("validator_update_action_change"))
    }

    fn ibc_acknowledgement_failure_change() -> Self {
        Self(ErrorKind::FieldNotSet("ibc_acknowledgement_failure_change"))
    }

    fn no_price_feed_genesis() -> Self {
        Self(ErrorKind::FieldNotSet("price_feed_change.genesis"))
    }

    fn price_feed_genesis(source: PriceFeedGenesisError) -> Self {
        Self(ErrorKind::PriceFeedGenesis {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("`{0}` field was not set")]
    FieldNotSet(&'static str),
    #[error("`price_feed_change.genesis` field was invalid")]
    PriceFeedGenesis { source: PriceFeedGenesisError },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        protocol::test_utils::dummy_price_feed_genesis,
        upgrades::v1::change::DeterministicSerialize,
    };

    #[test]
    fn serialized_price_feed_change_should_not_change() {
        let price_feed_change = PriceFeedChange {
            activation_height: 10,
            app_version: 2,
            genesis: Arc::new(dummy_price_feed_genesis()),
        };
        let serialized_price_feed_change = hex::encode(price_feed_change.to_vec());
        insta::assert_snapshot!("price_feed_change", serialized_price_feed_change);
    }

    #[test]
    fn serialized_validator_update_action_change_should_not_change() {
        let validator_update_action_change = ValidatorUpdateActionChange {
            activation_height: 10,
            app_version: 2,
        };
        let serialized_validator_update_action_change =
            hex::encode(validator_update_action_change.to_vec());
        insta::assert_snapshot!(
            "validator_update_action_change",
            serialized_validator_update_action_change
        );
    }

    #[test]
    fn serialized_ibc_acknowledgement_failure_change_should_not_change() {
        let ibc_acknowledgement_failure_change = IbcAcknowledgementFailureChange {
            activation_height: 10,
            app_version: 2,
        };
        let serialized_ibc_acknowledgement_failure_change =
            hex::encode(ibc_acknowledgement_failure_change.to_vec());
        insta::assert_snapshot!(
            "ibc_acknowledgement_failure_change",
            serialized_ibc_acknowledgement_failure_change
        );
    }
}
