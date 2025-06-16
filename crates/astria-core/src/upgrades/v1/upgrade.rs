use super::{
    Aspen,
    Change,
    UpgradeName,
};
use crate::upgrades::v1::blackburn::Blackburn;

/// An enum of the closed set of all possible upgrades.
#[derive(Clone, Debug)]
pub enum Upgrade {
    Aspen(Aspen),
    Blackburn(Blackburn),
}

impl Upgrade {
    #[must_use]
    pub fn activation_height(&self) -> u64 {
        match self {
            Upgrade::Aspen(aspen) => aspen.activation_height(),
            Upgrade::Blackburn(blackburn) => blackburn.activation_height(),
        }
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        match self {
            Upgrade::Aspen(aspen) => aspen.app_version(),
            Upgrade::Blackburn(blackburn) => blackburn.app_version(),
        }
    }

    #[must_use]
    pub fn shutdown_required(&self) -> bool {
        match self {
            Upgrade::Aspen(_) | Upgrade::Blackburn(_) => false,
        }
    }

    #[must_use]
    pub fn name(&self) -> UpgradeName {
        match self {
            Upgrade::Aspen(_) => Aspen::NAME.clone(),
            Upgrade::Blackburn(_) => Blackburn::NAME.clone(),
        }
    }

    #[must_use]
    pub fn changes(&self) -> Box<dyn Iterator<Item = &'_ dyn Change> + '_> {
        match self {
            Upgrade::Aspen(aspen) => Box::new(aspen.changes()),
            Upgrade::Blackburn(blackburn) => Box::new(blackburn.changes()),
        }
    }
}
