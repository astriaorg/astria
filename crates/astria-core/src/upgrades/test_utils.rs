use super::v1::Upgrades;
use crate::{
    generated::upgrades::v1 as raw,
    protocol::test_utils::dummy_price_feed_genesis,
    Protobuf as _,
};

pub struct UpgradesBuilder {
    aspen_activation_height: Option<u64>,
}

impl UpgradesBuilder {
    /// Returns a new `UpgradesBuilder`.
    ///
    /// By default, Aspen is included with an activation height of 100.
    #[must_use]
    pub fn new() -> Self {
        Self {
            aspen_activation_height: Some(100),
        }
    }

    /// To exclude Aspen, provide `activation_height` as `None`.
    #[must_use]
    pub fn set_aspen(mut self, activation_height: Option<u64>) -> Self {
        self.aspen_activation_height = activation_height;
        self
    }

    #[must_use]
    pub fn build(self) -> Upgrades {
        let aspen = self
            .aspen_activation_height
            .map(|activation_height| raw::Aspen {
                base_info: Some(raw::BaseUpgradeInfo {
                    activation_height,
                    app_version: 2,
                }),
                price_feed_change: Some(raw::aspen::PriceFeedChange {
                    genesis: Some(dummy_price_feed_genesis().into_raw()),
                }),
                validator_update_action_change: Some(raw::aspen::ValidatorUpdateActionChange {}),
                ibc_acknowledgement_failure_change: Some(
                    raw::aspen::IbcAcknowledgementFailureChange {},
                ),
            });
        let raw_upgrades = raw::Upgrades {
            aspen,
        };
        Upgrades::from_raw(raw_upgrades)
    }
}

impl Default for UpgradesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
