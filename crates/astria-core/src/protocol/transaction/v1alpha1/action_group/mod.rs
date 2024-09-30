#[cfg(test)]
mod tests;

use std::fmt::{
    self,
    Debug,
};

use penumbra_ibc::IbcRelay;

use super::{
    action::{
        ActionName,
        BridgeLockAction,
        BridgeSudoChangeAction,
        BridgeUnlockAction,
        FeeAssetChangeAction,
        FeeChangeAction,
        IbcRelayerChangeAction,
        IbcSudoChangeAction,
        Ics20Withdrawal,
        InitBridgeAccountAction,
        SequenceAction,
        SudoAddressChangeAction,
        TransferAction,
        ValidatorUpdate,
    },
    Action,
};

trait BelongsToGroup {
    const GROUP: ActionGroup;
}

macro_rules! impl_belong_to_group {
    ($(($act:ty, $group:expr)),*$(,)?) => {
        $(
            impl BelongsToGroup for $act {
                const GROUP: ActionGroup = $group;
            }
        )*
    }
}

impl_belong_to_group!(
    (SequenceAction, ActionGroup::BundleableGeneral),
    (TransferAction, ActionGroup::BundleableGeneral),
    (ValidatorUpdate, ActionGroup::BundleableGeneral),
    (SudoAddressChangeAction, ActionGroup::UnbundleableSudo),
    (IbcRelayerChangeAction, ActionGroup::BundleableSudo),
    (Ics20Withdrawal, ActionGroup::BundleableGeneral),
    (InitBridgeAccountAction, ActionGroup::UnbundleableGeneral),
    (BridgeLockAction, ActionGroup::BundleableGeneral),
    (BridgeUnlockAction, ActionGroup::BundleableGeneral),
    (BridgeSudoChangeAction, ActionGroup::UnbundleableGeneral),
    (FeeChangeAction, ActionGroup::BundleableSudo),
    (FeeAssetChangeAction, ActionGroup::BundleableSudo),
    (IbcRelay, ActionGroup::BundleableGeneral),
    (IbcSudoChangeAction, ActionGroup::UnbundleableSudo),
);

impl Action {
    const fn group(&self) -> ActionGroup {
        match self {
            Action::Sequence(_) => SequenceAction::GROUP,
            Action::Transfer(_) => TransferAction::GROUP,
            Action::ValidatorUpdate(_) => ValidatorUpdate::GROUP,
            Action::SudoAddressChange(_) => SudoAddressChangeAction::GROUP,
            Action::IbcRelayerChange(_) => IbcRelayerChangeAction::GROUP,
            Action::Ics20Withdrawal(_) => Ics20Withdrawal::GROUP,
            Action::InitBridgeAccount(_) => InitBridgeAccountAction::GROUP,
            Action::BridgeLock(_) => BridgeLockAction::GROUP,
            Action::BridgeUnlock(_) => BridgeUnlockAction::GROUP,
            Action::BridgeSudoChange(_) => BridgeSudoChangeAction::GROUP,
            Action::FeeChange(_) => FeeChangeAction::GROUP,
            Action::FeeAssetChange(_) => FeeAssetChangeAction::GROUP,
            Action::Ibc(_) => IbcRelay::GROUP,
            Action::IbcSudoChange(_) => IbcSudoChangeAction::GROUP,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum ActionGroup {
    BundleableGeneral,
    UnbundleableGeneral,
    BundleableSudo,
    UnbundleableSudo,
}

impl ActionGroup {
    pub(super) fn is_bundleable(self) -> bool {
        matches!(
            self,
            ActionGroup::BundleableGeneral | ActionGroup::BundleableSudo
        )
    }

    pub(super) fn is_bundleable_sudo(self) -> bool {
        matches!(self, ActionGroup::BundleableSudo)
    }
}

impl fmt::Display for ActionGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionGroup::BundleableGeneral => write!(f, "bundleable general"),
            ActionGroup::UnbundleableGeneral => write!(f, "unbundleable general"),
            ActionGroup::BundleableSudo => write!(f, "bundleable sudo"),
            ActionGroup::UnbundleableSudo => write!(f, "unbundleable sudo"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn mixed(
        original_group: ActionGroup,
        additional_group: ActionGroup,
        action: &'static str,
    ) -> Self {
        Self(ErrorKind::Mixed {
            original_group,
            additional_group,
            action,
        })
    }

    fn not_bundleable(group: ActionGroup) -> Self {
        Self(ErrorKind::NotBundleable {
            group,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error(
        "input contains mixed `ActionGroup` types. original group: {original_group}, additional \
         group: {additional_group}, triggering action: {action}"
    )]
    Mixed {
        original_group: ActionGroup,
        additional_group: ActionGroup,
        action: &'static str,
    },
    #[error("attempted to create bundle with non bundleable `ActionGroup` type: {group}")]
    NotBundleable { group: ActionGroup },
}

#[derive(Clone, Debug, Default)]
pub(super) struct Actions {
    inner: Vec<Action>,
}

impl Actions {
    pub(super) fn actions(&self) -> &[Action] {
        &self.inner
    }

    #[must_use]
    pub(super) fn into_actions(self) -> Vec<Action> {
        self.inner
    }

    pub(super) fn group(&self) -> Option<ActionGroup> {
        self.inner.first().map(super::action::Action::group)
    }

    pub(super) fn try_from_list_of_actions(actions: Vec<Action>) -> Result<Self, Error> {
        let mut actions_iter = actions.iter();
        let group = match actions_iter.next() {
            Some(action) => action.group(),
            None => {
                // empty `actions`
                return Ok(Self::default());
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
            inner: actions,
        })
    }
}
