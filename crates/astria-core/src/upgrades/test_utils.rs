use super::v1::Upgrades;
use crate::{
    generated::upgrades::v1 as raw,
    protocol::test_utils::dummy_price_feed_genesis,
    Protobuf as _,
};

pub struct UpgradesBuilder {
    upgrade_1_activation_height: Option<u64>,
}

impl UpgradesBuilder {
    /// Returns a new `UpgradesBuilder`.
    ///
    /// By default, upgrade 1 is included with an activation height of 100.
    #[must_use]
    pub fn new() -> Self {
        Self {
            upgrade_1_activation_height: Some(100),
        }
    }

    /// To exclude Upgrade 1, provide `activation_height` as `None`.
    #[must_use]
    pub fn set_upgrade_1(mut self, activation_height: Option<u64>) -> Self {
        self.upgrade_1_activation_height = activation_height;
        self
    }

    #[must_use]
    pub fn build(self) -> Upgrades {
        let upgrade_1 = self
            .upgrade_1_activation_height
            .map(|activation_height| raw::Upgrade1 {
                base_info: Some(raw::BaseUpgradeInfo {
                    activation_height,
                    app_version: 2,
                }),
                price_feed_change: Some(raw::upgrade1::PriceFeedChange {
                    genesis: Some(dummy_price_feed_genesis().into_raw()),
                }),
                validator_update_action_change: Some(raw::upgrade1::ValidatorUpdateActionChange {}),
            });
        let raw_upgrades = raw::Upgrades {
            upgrade_1,
        };
        Upgrades::from_raw(raw_upgrades)
    }
}

impl Default for UpgradesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
