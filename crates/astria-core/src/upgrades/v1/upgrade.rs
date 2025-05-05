use super::{
    Aspen,
    Change,
    UpgradeName,
};

/// An enum of the closed set of all possible upgrades.
#[derive(Clone, Debug)]
pub enum Upgrade {
    Aspen(Aspen),
}

impl Upgrade {
    #[must_use]
    pub fn activation_height(&self) -> u64 {
        match self {
            Upgrade::Aspen(aspen) => aspen.activation_height(),
        }
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        match self {
            Upgrade::Aspen(aspen) => aspen.app_version(),
        }
    }

    #[must_use]
    pub fn shutdown_required(&self) -> bool {
        match self {
            Upgrade::Aspen(_) => false,
        }
    }

    #[must_use]
    pub fn name(&self) -> UpgradeName {
        match self {
            Upgrade::Aspen(_) => Aspen::NAME.clone(),
        }
    }

    pub fn changes(&self) -> impl Iterator<Item = &'_ dyn Change> {
        match self {
            Upgrade::Aspen(aspen) => aspen.changes(),
        }
    }
}
