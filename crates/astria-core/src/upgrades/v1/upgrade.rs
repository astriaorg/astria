use super::{
    Change,
    Upgrade1,
    UpgradeName,
};

/// An enum of the closed set of all possible upgrades.
#[derive(Clone, Debug)]
pub enum Upgrade {
    Upgrade1(Upgrade1),
}

impl Upgrade {
    #[must_use]
    pub fn activation_height(&self) -> u64 {
        match self {
            Upgrade::Upgrade1(upgrade_1) => upgrade_1.activation_height(),
        }
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        match self {
            Upgrade::Upgrade1(upgrade_1) => upgrade_1.app_version(),
        }
    }

    #[must_use]
    pub fn shutdown_required(&self) -> bool {
        match self {
            Upgrade::Upgrade1(_) => false,
        }
    }

    #[must_use]
    pub fn name(&self) -> UpgradeName {
        match self {
            Upgrade::Upgrade1(_) => Upgrade1::NAME.clone(),
        }
    }

    pub fn changes(&self) -> impl Iterator<Item = &'_ dyn Change> {
        match self {
            Upgrade::Upgrade1(upgrade_1) => upgrade_1.changes(),
        }
    }
}
