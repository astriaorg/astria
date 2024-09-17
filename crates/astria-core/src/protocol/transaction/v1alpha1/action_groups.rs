use penumbra_ibc::IbcRelay;

use super::{
    action::{
        ActionError,
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
    raw,
};
use crate::Protobuf;
// Used to determine the type of action group to create
pub enum ActionGroupType {
    BundlableGeneral,
    General,
    BundlableSudo,
    Sudo,
}

pub trait ActionGroupTypeConverter {
    fn to_action_group(&self) -> ActionGroupType;
}

use raw::action::Value;

impl ActionGroupTypeConverter for Value {
    fn to_action_group(&self) -> ActionGroupType {
        use raw::action::Value;
        match self {
            Value::SequenceAction(_)
            | Value::TransferAction(_)
            | Value::ValidatorUpdateAction(_)
            | Value::IbcAction(_)
            | Value::Ics20Withdrawal(_)
            | Value::BridgeLockAction(_)
            | Value::BridgeUnlockAction(_) => ActionGroupType::BundlableGeneral,
            Value::SudoAddressChangeAction(_) => ActionGroupType::Sudo,
            Value::InitBridgeAccountAction(_) | Value::BridgeSudoChangeAction(_) => {
                ActionGroupType::General
            }
            Value::IbcRelayerChangeAction(_)
            | Value::FeeAssetChangeAction(_)
            | Value::FeeChangeAction(_) => ActionGroupType::BundlableSudo,
        }
    }
}

impl ActionGroup {
    #[allow(clippy::too_many_lines)] // TODO: refactor if reviewers request
    pub(super) fn try_from_raw(mut actions: Vec<raw::Action>) -> Result<Self, ActionGroupError> {
        let first_action = actions.first().ok_or_else(ActionGroupError::empty)?;
        let raw::Action {
            value,
        } = first_action;
        let Some(value_ref) = value else {
            return Err(ActionGroupError::action(ActionError::unset()));
        };

        let group_type = value_ref.to_action_group();

        match group_type {
            ActionGroupType::General => {
                if actions.len() > 1 {
                    return Err(ActionGroupError::not_bundlable());
                }
                let action = actions.pop().ok_or_else(ActionGroupError::empty)?;
                let Some(value) = action.value else {
                    return Err(ActionGroupError::action(ActionError::unset()));
                };
                match value {
                    Value::InitBridgeAccountAction(act) => Ok(ActionGroup::General(General {
                        actions: GeneralAction::InitBridgeAccount(
                            InitBridgeAccountAction::try_from_raw(act)
                                .map_err(ActionError::init_bridge_account)
                                .map_err(ActionGroupError::action)?,
                        ),
                    })),
                    Value::BridgeSudoChangeAction(act) => Ok(ActionGroup::General(General {
                        actions: GeneralAction::BridgeSudoChange(
                            BridgeSudoChangeAction::try_from_raw(act)
                                .map_err(ActionError::bridge_sudo_change)
                                .map_err(ActionGroupError::action)?,
                        ),
                    })),
                    _ => Err(ActionGroupError::mixed()),
                }
            }
            ActionGroupType::Sudo => {
                if actions.len() > 1 {
                    return Err(ActionGroupError::not_bundlable());
                }
                let action = actions.pop().ok_or_else(ActionGroupError::empty)?;
                let Some(value) = action.value else {
                    return Err(ActionGroupError::action(ActionError::unset()));
                };
                match value {
                    Value::SudoAddressChangeAction(act) => Ok(ActionGroup::Sudo(Sudo {
                        actions: SudoAction::SudoAddressChange(
                            SudoAddressChangeAction::try_from_raw(act)
                                .map_err(ActionError::sudo_address_change)
                                .map_err(ActionGroupError::action)?,
                        ),
                    })),
                    _ => Err(ActionGroupError::mixed()),
                }
            }
            ActionGroupType::BundlableSudo => {
                let mut sudo_bundle_actions = Vec::<BundlableSudoAction>::new();
                for action in actions {
                    let Some(value) = action.value else {
                        return Err(ActionGroupError::action(ActionError::unset()));
                    };
                    match value {
                        Value::IbcRelayerChangeAction(act) => {
                            sudo_bundle_actions.push(BundlableSudoAction::IbcRelayerChange(
                                IbcRelayerChangeAction::try_from_raw(act)
                                    .map_err(ActionError::ibc_relayer_change)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::FeeAssetChangeAction(act) => {
                            sudo_bundle_actions.push(BundlableSudoAction::FeeAssetChange(
                                FeeAssetChangeAction::try_from_raw(act)
                                    .map_err(ActionError::fee_asset_change)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::FeeChangeAction(act) => {
                            sudo_bundle_actions.push(BundlableSudoAction::FeeChange(
                                FeeChangeAction::try_from_raw(act)
                                    .map_err(ActionError::fee_change)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        _ => return Err(ActionGroupError::mixed()),
                    }
                }
                Ok(ActionGroup::BundlableSudo(BundlableSudo {
                    actions: sudo_bundle_actions,
                }))
            }
            ActionGroupType::BundlableGeneral => {
                let mut bundlable_general_actions = Vec::<BundlableGeneralAction>::new();
                for action in actions {
                    let Some(value) = action.value else {
                        return Err(ActionGroupError::action(ActionError::unset()));
                    };
                    match value {
                        Value::SequenceAction(act) => {
                            bundlable_general_actions.push(BundlableGeneralAction::Sequence(
                                SequenceAction::try_from_raw(act)
                                    .map_err(ActionError::sequence)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::TransferAction(act) => {
                            bundlable_general_actions.push(BundlableGeneralAction::Transfer(
                                TransferAction::try_from_raw(act)
                                    .map_err(ActionError::transfer)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::IbcAction(act) => {
                            bundlable_general_actions.push(BundlableGeneralAction::Ibc(
                                IbcRelay::try_from(act)
                                    .map_err(|e| ActionError::ibc(e.into()))
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::Ics20Withdrawal(act) => {
                            bundlable_general_actions.push(
                                BundlableGeneralAction::Ics20Withdrawal(
                                    Ics20Withdrawal::try_from_raw(act)
                                        .map_err(ActionError::ics20_withdrawal)
                                        .map_err(ActionGroupError::action)?,
                                ),
                            );
                        }
                        Value::BridgeLockAction(act) => {
                            bundlable_general_actions.push(BundlableGeneralAction::BridgeLock(
                                BridgeLockAction::try_from_raw(act)
                                    .map_err(ActionError::bridge_lock)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::BridgeUnlockAction(act) => {
                            bundlable_general_actions.push(BundlableGeneralAction::BridgeUnlock(
                                BridgeUnlockAction::try_from_raw(act)
                                    .map_err(ActionError::bridge_unlock)
                                    .map_err(ActionGroupError::action)?,
                            ));
                        }
                        Value::ValidatorUpdateAction(act) => {
                            bundlable_general_actions.push(
                                BundlableGeneralAction::ValidatorUpdate(
                                    ValidatorUpdate::try_from_raw(act)
                                        .map_err(ActionError::validator_update)
                                        .map_err(ActionGroupError::action)?,
                                ),
                            );
                        }
                        _ => return Err(ActionGroupError::mixed()),
                    }
                }
                Ok(ActionGroup::BundlableGeneral(BundlableGeneral {
                    actions: bundlable_general_actions,
                }))
            }
        }
    }

    pub(super) fn to_raw_protobuf_mut(&mut self) -> Vec<raw::Action> {
        match self {
            ActionGroup::Sudo(sudo) => {
                let action = &sudo.actions;
                match action {
                    SudoAction::SudoAddressChange(act) => {
                        vec![raw::Action {
                            value: Some(Value::SudoAddressChangeAction(act.to_raw())),
                        }]
                    }
                }
            }
            ActionGroup::General(general) => {
                let action = &general.actions;
                match action {
                    GeneralAction::InitBridgeAccount(act) => {
                        vec![raw::Action {
                            value: Some(Value::InitBridgeAccountAction(act.to_raw())),
                        }]
                    }
                    GeneralAction::BridgeSudoChange(act) => {
                        vec![raw::Action {
                            value: Some(Value::BridgeSudoChangeAction(act.to_raw())),
                        }]
                    }
                }
            }
            ActionGroup::BundlableGeneral(bundlable_general) => {
                let mut actions = Vec::<raw::Action>::new();
                for action in &mut bundlable_general.actions {
                    match action {
                        BundlableGeneralAction::Sequence(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::SequenceAction(act.to_raw())),
                            });
                        }
                        BundlableGeneralAction::Transfer(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::TransferAction(act.to_raw())),
                            });
                        }
                        BundlableGeneralAction::Ibc(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::IbcAction(act.clone().into())),
                            });
                        }
                        BundlableGeneralAction::Ics20Withdrawal(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::Ics20Withdrawal(act.to_raw())),
                            });
                        }
                        BundlableGeneralAction::BridgeLock(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::BridgeLockAction(act.to_raw())),
                            });
                        }
                        BundlableGeneralAction::BridgeUnlock(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::BridgeUnlockAction(act.to_raw())),
                            });
                        }
                        BundlableGeneralAction::ValidatorUpdate(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::ValidatorUpdateAction(act.to_raw())),
                            });
                        }
                    }
                }
                actions
            }
            ActionGroup::BundlableSudo(bundleable_sudo) => {
                let mut actions = Vec::<raw::Action>::new();
                for action in &mut bundleable_sudo.actions {
                    match action {
                        BundlableSudoAction::IbcRelayerChange(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::IbcRelayerChangeAction(act.to_raw())),
                            });
                        }
                        BundlableSudoAction::FeeAssetChange(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::FeeAssetChangeAction(act.to_raw())),
                            });
                        }
                        BundlableSudoAction::FeeChange(act) => {
                            actions.push(raw::Action {
                                value: Some(Value::FeeChangeAction(act.to_raw())),
                            });
                        }
                    }
                }
                actions
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ActionGroupErrorKind {
    #[error("`actions` field is invalid")]
    Action(#[source] ActionError),
    #[error("no actions provided")]
    Empty,
    #[error("input contains mixed action types")]
    Mixed,
    #[error("input attempted to bundle non bundleable action type")]
    NotBundleable,
    #[error("tried to convert from wrong ActionGroup type for inner action")]
    WrongActionGroupType,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ActionGroupError(ActionGroupErrorKind);
impl ActionGroupError {
    fn action(inner: ActionError) -> Self {
        Self(ActionGroupErrorKind::Action(inner))
    }

    fn empty() -> Self {
        Self(ActionGroupErrorKind::Empty)
    }

    fn mixed() -> Self {
        Self(ActionGroupErrorKind::Mixed)
    }

    fn not_bundlable() -> Self {
        Self(ActionGroupErrorKind::NotBundleable)
    }

    fn wrong_action_group_type() -> Self {
        Self(ActionGroupErrorKind::WrongActionGroupType)
    }
}

#[allow(clippy::large_enum_variant)] // TODO: figure out if this is a problem
#[derive(Clone, Debug)]
pub enum ActionGroup {
    BundlableGeneral(BundlableGeneral),
    General(General),
    BundlableSudo(BundlableSudo),
    Sudo(Sudo),
}

#[derive(Clone, Debug)]
pub struct BundlableGeneral {
    pub actions: Vec<BundlableGeneralAction>,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize, ::serde::Serialize),
    serde(into = "raw::Action", try_from = "raw::Action")
)]
pub enum BundlableGeneralAction {
    Sequence(SequenceAction),
    Transfer(TransferAction),
    Ibc(IbcRelay),
    Ics20Withdrawal(Ics20Withdrawal),
    BridgeLock(BridgeLockAction),
    BridgeUnlock(BridgeUnlockAction),
    ValidatorUpdate(ValidatorUpdate),
}

impl From<BundlableGeneralAction> for raw::Action {
    fn from(value: BundlableGeneralAction) -> Self {
        use raw::action::Value;
        match value {
            BundlableGeneralAction::Sequence(act) => Self {
                value: Some(Value::SequenceAction(act.into_raw())),
            },
            BundlableGeneralAction::Transfer(act) => Self {
                value: Some(Value::TransferAction(act.to_raw())),
            },
            BundlableGeneralAction::Ibc(act) => Self {
                value: Some(Value::IbcAction(act.into())),
            },
            BundlableGeneralAction::Ics20Withdrawal(act) => Self {
                value: Some(Value::Ics20Withdrawal(act.to_raw())),
            },
            BundlableGeneralAction::BridgeLock(act) => Self {
                value: Some(Value::BridgeLockAction(act.to_raw())),
            },
            BundlableGeneralAction::BridgeUnlock(act) => Self {
                value: Some(Value::BridgeUnlockAction(act.to_raw())),
            },
            BundlableGeneralAction::ValidatorUpdate(act) => Self {
                value: Some(Value::ValidatorUpdateAction(act.to_raw())),
            },
        }
    }
}

impl TryFrom<raw::Action> for BundlableGeneralAction {
    type Error = ActionGroupError;

    fn try_from(value: raw::Action) -> Result<Self, Self::Error> {
        match value.value {
            Some(raw::action::Value::SequenceAction(act)) => Ok(BundlableGeneralAction::Sequence(
                SequenceAction::try_from_raw(act)
                    .map_err(ActionError::sequence)
                    .map_err(ActionGroupError::action)?,
            )),
            Some(raw::action::Value::TransferAction(act)) => Ok(BundlableGeneralAction::Transfer(
                TransferAction::try_from_raw(act)
                    .map_err(ActionError::transfer)
                    .map_err(ActionGroupError::action)?,
            )),
            Some(raw::action::Value::IbcAction(act)) => Ok(BundlableGeneralAction::Ibc(
                IbcRelay::try_from(act)
                    .map_err(|e| ActionError::ibc(e.into()))
                    .map_err(ActionGroupError::action)?,
            )),
            Some(raw::action::Value::Ics20Withdrawal(act)) => {
                Ok(BundlableGeneralAction::Ics20Withdrawal(
                    Ics20Withdrawal::try_from_raw(act)
                        .map_err(ActionError::ics20_withdrawal)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            Some(raw::action::Value::BridgeLockAction(act)) => {
                Ok(BundlableGeneralAction::BridgeLock(
                    BridgeLockAction::try_from_raw(act)
                        .map_err(ActionError::bridge_lock)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            Some(raw::action::Value::BridgeUnlockAction(act)) => {
                Ok(BundlableGeneralAction::BridgeUnlock(
                    BridgeUnlockAction::try_from_raw(act)
                        .map_err(ActionError::bridge_unlock)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            Some(raw::action::Value::ValidatorUpdateAction(act)) => {
                Ok(BundlableGeneralAction::ValidatorUpdate(
                    ValidatorUpdate::try_from_raw(act)
                        .map_err(ActionError::validator_update)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            _ => Err(ActionGroupError::wrong_action_group_type()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct General {
    pub actions: GeneralAction,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize, ::serde::Serialize),
    serde(into = "raw::Action", try_from = "raw::Action")
)]
pub enum GeneralAction {
    InitBridgeAccount(InitBridgeAccountAction),
    BridgeSudoChange(BridgeSudoChangeAction),
}

impl From<GeneralAction> for raw::Action {
    fn from(value: GeneralAction) -> Self {
        use raw::action::Value;
        match value {
            GeneralAction::InitBridgeAccount(act) => Self {
                value: Some(Value::InitBridgeAccountAction(act.to_raw())),
            },
            GeneralAction::BridgeSudoChange(act) => Self {
                value: Some(Value::BridgeSudoChangeAction(act.to_raw())),
            },
        }
    }
}

impl TryFrom<raw::Action> for GeneralAction {
    type Error = ActionGroupError;

    fn try_from(value: raw::Action) -> Result<Self, Self::Error> {
        match value.value {
            Some(raw::action::Value::InitBridgeAccountAction(act)) => {
                Ok(GeneralAction::InitBridgeAccount(
                    InitBridgeAccountAction::try_from_raw(act)
                        .map_err(ActionError::init_bridge_account)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            Some(raw::action::Value::BridgeSudoChangeAction(act)) => {
                Ok(GeneralAction::BridgeSudoChange(
                    BridgeSudoChangeAction::try_from_raw(act)
                        .map_err(ActionError::bridge_sudo_change)
                        .map_err(ActionGroupError::action)?,
                ))
            }
            _ => Err(ActionGroupError::wrong_action_group_type()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BundlableSudo {
    pub actions: Vec<BundlableSudoAction>,
}

#[derive(Clone, Debug)]
pub enum BundlableSudoAction {
    IbcRelayerChange(IbcRelayerChangeAction),
    FeeAssetChange(FeeAssetChangeAction),
    FeeChange(FeeChangeAction),
}

#[derive(Clone, Debug)]
pub struct Sudo {
    pub actions: SudoAction,
}

#[derive(Clone, Debug)]
pub enum SudoAction {
    SudoAddressChange(SudoAddressChangeAction),
}

impl From<BundlableGeneral> for ActionGroup {
    fn from(bundlable_general: BundlableGeneral) -> Self {
        ActionGroup::BundlableGeneral(bundlable_general)
    }
}

impl From<BundlableSudo> for ActionGroup {
    fn from(bundlable_sudo: BundlableSudo) -> Self {
        ActionGroup::BundlableSudo(bundlable_sudo)
    }
}

impl From<General> for ActionGroup {
    fn from(general: General) -> Self {
        ActionGroup::General(general)
    }
}

impl From<Sudo> for ActionGroup {
    fn from(sudo: Sudo) -> Self {
        ActionGroup::Sudo(sudo)
    }
}
