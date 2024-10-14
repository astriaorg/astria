#[cfg(test)]
mod tests;

use std::fmt::{
    self,
    Debug,
};

use penumbra_ibc::IbcRelay;

use super::{
    Action,
    ActionName,
    BridgeLock,
    BridgeSudoChange,
    BridgeUnlock,
    FeeAssetChange,
    FeeChangeKind,
    IbcRelayerChange,
    IbcSudoChange,
    Ics20Withdrawal,
    InitBridgeAccount,
    Sequence,
    SudoAddressChange,
    Transfer,
    ValidatorUpdate,
};

trait BelongsToGroup {
    const GROUP: Group;
}

macro_rules! impl_belong_to_group {
    ($(($act:ty, $group:expr)),*$(,)?) => {
        $(
            impl BelongsToGroup for $act {
                const GROUP: Group = $group;
            }
        )*
    }
}

impl_belong_to_group!(
    (Sequence, Group::BundleableGeneral),
    (Transfer, Group::BundleableGeneral),
    (ValidatorUpdate, Group::BundleableGeneral),
    (SudoAddressChange, Group::UnbundleableSudo),
    (IbcRelayerChange, Group::BundleableSudo),
    (Ics20Withdrawal, Group::BundleableGeneral),
    (InitBridgeAccount, Group::UnbundleableGeneral),
    (BridgeLock, Group::BundleableGeneral),
    (BridgeUnlock, Group::BundleableGeneral),
    (BridgeSudoChange, Group::UnbundleableGeneral),
    (FeeChangeKind, Group::BundleableSudo),
    (FeeAssetChange, Group::BundleableSudo),
    (IbcRelay, Group::BundleableGeneral),
    (IbcSudoChange, Group::UnbundleableSudo),
);

impl Action {
    pub const fn group(&self) -> Group {
        match self {
            Action::Sequence(_) => Sequence::GROUP,
            Action::Transfer(_) => Transfer::GROUP,
            Action::ValidatorUpdate(_) => ValidatorUpdate::GROUP,
            Action::SudoAddressChange(_) => SudoAddressChange::GROUP,
            Action::IbcRelayerChange(_) => IbcRelayerChange::GROUP,
            Action::Ics20Withdrawal(_) => Ics20Withdrawal::GROUP,
            Action::InitBridgeAccount(_) => InitBridgeAccount::GROUP,
            Action::BridgeLock(_) => BridgeLock::GROUP,
            Action::BridgeUnlock(_) => BridgeUnlock::GROUP,
            Action::BridgeSudoChange(_) => BridgeSudoChange::GROUP,
            Action::FeeChange(_) => FeeChangeKind::GROUP,
            Action::FeeAssetChange(_) => FeeAssetChange::GROUP,
            Action::Ibc(_) => IbcRelay::GROUP,
            Action::IbcSudoChange(_) => IbcSudoChange::GROUP,
        }
    }
}

/// `action::Group`
///
/// Used to constrain the types of actions that can be included in a single
/// transaction and the order which transactions are ran in a block.
///
/// NOTE: The ordering is important and must be maintained.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    UnbundleableSudo = 1,
    BundleableSudo = 2,
    UnbundleableGeneral = 3,
    BundleableGeneral = 4,
}

impl Group {
    pub(crate) fn is_bundleable(self) -> bool {
        matches!(self, Group::BundleableGeneral | Group::BundleableSudo)
    }

    pub(crate) fn is_bundleable_sudo(self) -> bool {
        matches!(self, Group::BundleableSudo)
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Group::BundleableGeneral => write!(f, "bundleable general"),
            Group::UnbundleableGeneral => write!(f, "unbundleable general"),
            Group::BundleableSudo => write!(f, "bundleable sudo"),
            Group::UnbundleableSudo => write!(f, "unbundleable sudo"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn mixed(original_group: Group, additional_group: Group, action: &'static str) -> Self {
        Self(ErrorKind::Mixed {
            original_group,
            additional_group,
            action,
        })
    }

    fn not_bundleable(group: Group) -> Self {
        Self(ErrorKind::NotBundleable {
            group,
        })
    }

    fn empty() -> Self {
        Self(ErrorKind::Empty)
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error(
        "input contains mixed `Group` types. original group: {original_group}, additional group: \
         {additional_group}, triggering action: {action}"
    )]
    Mixed {
        original_group: Group,
        additional_group: Group,
        action: &'static str,
    },
    #[error("attempted to create bundle with non bundleable `Group` type: {group}")]
    NotBundleable { group: Group },
    #[error("actions cannot be empty")]
    Empty,
}

#[derive(Clone, Debug)]
pub(crate) struct Actions {
    group: Group,
    inner: Vec<Action>,
}

impl Actions {
    pub(crate) fn actions(&self) -> &[Action] {
        &self.inner
    }

    #[must_use]
    pub(crate) fn into_actions(self) -> Vec<Action> {
        self.inner
    }

    pub(crate) fn group(&self) -> Group {
        self.group
    }

    pub(crate) fn try_from_list_of_actions(actions: Vec<Action>) -> Result<Self, Error> {
        let mut actions_iter = actions.iter();
        let group = match actions_iter.next() {
            Some(action) => action.group(),
            None => {
                // empty `actions`
                return Err(Error::empty());
            }
        };

        // assert size constraints on non-bundleable action groups
        if actions.len() > 1 && !group.is_bundleable() {
            return Err(Error::not_bundleable(group));
        }

        // assert the rest of the actions have the same group as the first
        for action in actions_iter {
            if action.group() != group {
                return Err(Error::mixed(group, action.group(), action.name()));
            }
        }

        Ok(Self {
            group,
            inner: actions,
        })
    }
}
