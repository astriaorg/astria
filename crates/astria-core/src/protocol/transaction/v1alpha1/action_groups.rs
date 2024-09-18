use super::{
    action::{
        BridgeLockAction,
        BridgeSudoChangeAction,
        BridgeUnlockAction,
        FeeAssetChangeAction,
        FeeChangeAction,
        IbcRelayerChangeAction,
        Ics20Withdrawal,
        InitBridgeAccountAction,
        SequenceAction,
        SudoAddressChangeAction,
        TransferAction,
        ValidatorUpdate,
    },
    Action,
};
trait Sealed {}

trait BelongsToGroup: Sealed {
    fn belongs_to_group(&self) -> ActionGroup;
}

macro_rules! impl_belong_to_group {
    ($($act:ty),*$(,)?) => {
        $(
            impl Sealed for $act {}

            impl BelongsToGroup for $act {
                fn belongs_to_group(&self) -> ActionGroup {
                    Self::ACTION_GROUP.into()
                }
            }
        )*
    }
}

impl SequenceAction {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl TransferAction {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl ValidatorUpdate {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl SudoAddressChangeAction {
    const ACTION_GROUP: Sudo = Sudo;
}

impl IbcRelayerChangeAction {
    const ACTION_GROUP: BundlableSudo = BundlableSudo;
}

impl Ics20Withdrawal {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl FeeAssetChangeAction {
    const ACTION_GROUP: BundlableSudo = BundlableSudo;
}

impl InitBridgeAccountAction {
    const ACTION_GROUP: General = General;
}

impl BridgeLockAction {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl BridgeUnlockAction {
    const ACTION_GROUP: BundlableGeneral = BundlableGeneral;
}

impl BridgeSudoChangeAction {
    const ACTION_GROUP: General = General;
}

impl FeeChangeAction {
    const ACTION_GROUP: BundlableSudo = BundlableSudo;
}

impl_belong_to_group!(
    SequenceAction,
    TransferAction,
    ValidatorUpdate,
    SudoAddressChangeAction,
    IbcRelayerChangeAction,
    Ics20Withdrawal,
    InitBridgeAccountAction,
    BridgeLockAction,
    BridgeUnlockAction,
    BridgeSudoChangeAction,
    FeeChangeAction,
    FeeAssetChangeAction
);

impl Sealed for Action {}
impl BelongsToGroup for Action {
    fn belongs_to_group(&self) -> ActionGroup {
        match self {
            Action::Sequence(act) => act.belongs_to_group(),
            Action::Transfer(act) => act.belongs_to_group(),
            Action::ValidatorUpdate(act) => act.belongs_to_group(),
            Action::SudoAddressChange(act) => act.belongs_to_group(),
            Action::IbcRelayerChange(act) => act.belongs_to_group(),
            Action::Ics20Withdrawal(act) => act.belongs_to_group(),
            Action::InitBridgeAccount(act) => act.belongs_to_group(),
            Action::BridgeLock(act) => act.belongs_to_group(),
            Action::BridgeUnlock(act) => act.belongs_to_group(),
            Action::BridgeSudoChange(act) => act.belongs_to_group(),
            Action::FeeChange(act) => act.belongs_to_group(),
            Action::FeeAssetChange(act) => act.belongs_to_group(),
            Action::Ibc(_) => BundlableGeneral.into(), /* Can't implement on action directly
                                                        * since it lives in a external crate */
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlableGeneral;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct General;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlableSudo;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sudo;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionGroup {
    BundlableGeneral(BundlableGeneral),
    General(General),
    BundlableSudo(BundlableSudo),
    Sudo(Sudo),
}

impl From<BundlableGeneral> for ActionGroup {
    fn from(val: BundlableGeneral) -> ActionGroup {
        ActionGroup::BundlableGeneral(val)
    }
}

impl From<General> for ActionGroup {
    fn from(val: General) -> ActionGroup {
        ActionGroup::General(val)
    }
}

impl From<BundlableSudo> for ActionGroup {
    fn from(val: BundlableSudo) -> ActionGroup {
        ActionGroup::BundlableSudo(val)
    }
}

impl From<Sudo> for ActionGroup {
    fn from(val: Sudo) -> ActionGroup {
        ActionGroup::Sudo(val)
    }
}

#[derive(Debug, thiserror::Error)]
enum ActionGroupErrorKind {
    #[error("input contains mixed action types")]
    Mixed,
    #[error("input attempted to bundle non bundleable action type")]
    NotBundleable,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ActionGroupError(ActionGroupErrorKind);
impl ActionGroupError {
    fn mixed() -> Self {
        Self(ActionGroupErrorKind::Mixed)
    }

    fn not_bundlable() -> Self {
        Self(ActionGroupErrorKind::NotBundleable)
    }
}

/// Invariants: `group` is set if `inner` is not empty.
#[derive(Clone, Debug)]
pub(super) struct Actions {
    group: Option<ActionGroup>,
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

    pub(super) fn group(&self) -> &Option<ActionGroup> {
        &self.group
    }

    pub(super) fn from_list_of_actions(actions: Vec<Action>) -> Result<Self, ActionGroupError> {
        let mut group = None;
        for action in &actions {
            if group.is_none() {
                group = Some(action.belongs_to_group());
            } else if group != Some(action.belongs_to_group()) {
                return Err(ActionGroupError::mixed());
            }
        }

        // assert size constraints on non-bundlable action groups
        if (Some(ActionGroup::General(General)) == group || Some(ActionGroup::Sudo(Sudo)) == group)
            && actions.len() > 1
        {
            return Err(ActionGroupError::not_bundlable());
        }
        Ok(Self {
            group,
            inner: actions,
        })
    }
}
