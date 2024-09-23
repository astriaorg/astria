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
    (
        SequenceAction,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (
        TransferAction,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (
        ValidatorUpdate,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (SudoAddressChangeAction, ActionGroup::Sudo(Sudo)),
    (
        IbcRelayerChangeAction,
        ActionGroup::BundlableSudo(BundlableSudo)
    ),
    (
        Ics20Withdrawal,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (InitBridgeAccountAction, ActionGroup::General(General)),
    (
        BridgeLockAction,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (
        BridgeUnlockAction,
        ActionGroup::BundlableGeneral(BundlableGeneral)
    ),
    (BridgeSudoChangeAction, ActionGroup::General(General)),
    (FeeChangeAction, ActionGroup::BundlableSudo(BundlableSudo)),
    (
        FeeAssetChangeAction,
        ActionGroup::BundlableSudo(BundlableSudo)
    ),
    (IbcRelay, ActionGroup::BundlableGeneral(BundlableGeneral)),
);

pub trait Group {
    fn group(&self) -> ActionGroup;
}

impl Group for Action {
    fn group(&self) -> ActionGroup {
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
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BundlableGeneral;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct General;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BundlableSudo;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Sudo;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActionGroup {
    BundlableGeneral(BundlableGeneral),
    General(General),
    BundlableSudo(BundlableSudo),
    Sudo(Sudo),
}

impl fmt::Display for ActionGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionGroup::BundlableGeneral(_) => write!(f, "BundlableGeneral"),
            ActionGroup::General(_) => write!(f, "General"),
            ActionGroup::BundlableSudo(_) => write!(f, "BundlableSudo"),
            ActionGroup::Sudo(_) => write!(f, "Sudo"),
        }
    }
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
enum ErrorKind {
    #[error("input contains mixed action types")]
    Mixed,
    #[error("attempted to create bundle with non bundleable `ActionGroup` type")]
    NotBundleable,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    context: Option<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(ctx) = &self.context {
            write!(f, ": {ctx}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error {
    fn new(kind: ErrorKind, context: Option<String>) -> Self {
        Self {
            kind,
            context,
        }
    }

    #[must_use]
    pub fn mixed(
        original_group: ActionGroup,
        additional_group: ActionGroup,
        action: &Action,
    ) -> Self {
        let context = format!(
            "Mixed actions of different types. Original group: '{original_group}', Additional \
             group: '{additional_group}', triggering action: '{}'",
            action.name()
        );
        Self::new(ErrorKind::Mixed, Some(context))
    }

    #[must_use]
    pub fn not_bundlable(group: ActionGroup) -> Self {
        let context = format!("ActionGroup type '{group}' is not bundleable");
        Self::new(ErrorKind::NotBundleable, Some(context))
    }
}

/// Invariants: `group` is set if `inner` is not empty.
#[derive(Clone, Debug)]
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
        self.inner.first().map(Group::group)
    }

    pub(super) fn default() -> Self {
        Self {
            inner: vec![],
        }
    }

    pub(super) fn from_list_of_actions(actions: Vec<Action>) -> Result<Self, Error> {
        let mut actions_iter = actions.iter();
        let group = match actions_iter.next() {
            Some(action) => action.group(),
            None => {
                // empty `actions`, so invariants met
                return Ok(Self::default());
            }
        };

        // assert size constraints on non-bundlable action groups
        if matches!(group, ActionGroup::General(_) | ActionGroup::Sudo(_)) && actions.len() > 1 {
            return Err(Error::not_bundlable(group));
        }

        // assert the rest of the actions have the same group as the first
        for action in actions_iter {
            if action.group() != group {
                return Err(Error::mixed(group, action.group(), action));
            }
        }

        Ok(Self {
            inner: actions,
        })
    }
}

#[cfg(test)]
mod test {
    use ibc_types::core::client::Height;

    use super::*;
    use crate::{
        crypto::VerificationKey,
        primitive::v1::{
            asset::Denom,
            Address,
            RollupId,
        },
        protocol::transaction::v1alpha1::action::{
            FeeChange,
            TransferAction,
        },
    };
    const ASTRIA_ADDRESS_PREFIX: &str = "astria";

    #[test]
    fn from_list_of_actions_bundlable_general() {
        let address: Address<_> = Address::builder()
            .array([0; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();

        let asset: Denom = "nria".parse().unwrap();
        let actions = vec![
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from([8; 32]),
                data: vec![].into(),
                fee_asset: asset.clone(),
            })
            .into(),
            Action::Transfer(TransferAction {
                to: address,
                amount: 100,
                asset: asset.clone(),
                fee_asset: asset.clone(),
            })
            .into(),
            Action::BridgeLock(BridgeLockAction {
                to: address,
                amount: 100,
                asset: asset.clone(),
                fee_asset: asset.clone(),
                destination_chain_address: "".to_string(),
            })
            .into(),
            Action::BridgeUnlock(BridgeUnlockAction {
                to: address,
                amount: 100,
                fee_asset: asset.clone(),
                bridge_address: address,
                memo: "".to_string(),
                rollup_block_number: 0,
                rollup_withdrawal_event_id: "".to_string(),
            })
            .into(),
            Action::ValidatorUpdate(ValidatorUpdate {
                power: 100,
                verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            })
            .into(),
            Action::Ics20Withdrawal(Ics20Withdrawal {
                denom: asset.clone(),
                destination_chain_address: "test".to_string(),
                return_address: address,
                amount: 1_000_000u128,
                memo: "test".to_string(),
                fee_asset: asset.clone(),
                timeout_height: Height::new(1, 1).unwrap(),
                timeout_time: 0,
                source_channel: "channel-0".parse().unwrap(),
                bridge_address: Some(address),
                use_compat_address: false,
            })
            .into(),
        ];

        assert!(matches!(
            Actions::from_list_of_actions(actions).unwrap().group(),
            Some(ActionGroup::BundlableGeneral(_))
        ));
    }

    #[test]
    fn from_list_of_actions_bundlable_sudo() {
        let address: Address<_> = Address::builder()
            .array([0; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();

        let asset: Denom = "nria".parse().unwrap();
        let actions = vec![
            Action::FeeChange(FeeChangeAction {
                fee_change: FeeChange::TransferBaseFee,
                new_value: 100,
            })
            .into(),
            Action::FeeAssetChange(FeeAssetChangeAction::Addition(asset).into()).into(),
            Action::IbcRelayerChange(IbcRelayerChangeAction::Addition(address)).into(),
        ];

        assert!(matches!(
            Actions::from_list_of_actions(actions).unwrap().group(),
            Some(ActionGroup::BundlableSudo(_))
        ));
    }

    #[test]
    fn from_list_of_actions_sudo() {
        let address: Address<_> = Address::builder()
            .array([0; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();

        let actions = vec![
            Action::SudoAddressChange(SudoAddressChangeAction {
                new_address: address,
            })
            .into(),
        ];

        assert!(matches!(
            Actions::from_list_of_actions(actions).unwrap().group(),
            Some(ActionGroup::Sudo(_))
        ));

        let actions = vec![
            Action::SudoAddressChange(SudoAddressChangeAction {
                new_address: address,
            })
            .into(),
            Action::SudoAddressChange(SudoAddressChangeAction {
                new_address: address,
            })
            .into(),
        ];

        assert_eq!(
            Actions::from_list_of_actions(actions)
                .unwrap_err()
                .to_string(),
            "attempted to create bundle with non bundleable `ActionGroup` type: ActionGroup type \
             'Sudo' is not bundleable"
        );
    }

    #[test]
    fn from_list_of_actions_general() {
        let address: Address<_> = Address::builder()
            .array([0; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();

        let asset: Denom = "nria".parse().unwrap();

        let init_bridge_account_action = InitBridgeAccountAction {
            rollup_id: RollupId::from([8; 32]),
            asset: asset.clone(),
            fee_asset: asset.clone(),
            sudo_address: Some(address),
            withdrawer_address: Some(address),
        };

        let sudo_bridge_address_change_action = BridgeSudoChangeAction {
            new_sudo_address: Some(address),
            bridge_address: address,
            new_withdrawer_address: Some(address),
            fee_asset: asset.clone(),
        };

        let actions = vec![init_bridge_account_action.clone().into()];

        assert!(matches!(
            Actions::from_list_of_actions(actions).unwrap().group(),
            Some(ActionGroup::General(_))
        ));

        let actions = vec![sudo_bridge_address_change_action.clone().into()];

        assert!(matches!(
            Actions::from_list_of_actions(actions).unwrap().group(),
            Some(ActionGroup::General(_))
        ));

        let actions = vec![
            init_bridge_account_action.into(),
            sudo_bridge_address_change_action.into(),
        ];

        assert_eq!(
            Actions::from_list_of_actions(actions)
                .unwrap_err()
                .to_string(),
            "attempted to create bundle with non bundleable `ActionGroup` type: ActionGroup type \
             'General' is not bundleable"
        );
    }

    #[test]
    fn from_list_of_actions_mixed() {
        let address: Address<_> = Address::builder()
            .array([0; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();

        let asset: Denom = "nria".parse().unwrap();
        let actions = vec![
            Action::Sequence(SequenceAction {
                rollup_id: RollupId::from([8; 32]),
                data: vec![].into(),
                fee_asset: asset.clone(),
            })
            .into(),
            Action::SudoAddressChange(SudoAddressChangeAction {
                new_address: address,
            })
            .into(),
        ];

        assert_eq!(
            Actions::from_list_of_actions(actions)
                .unwrap_err()
                .to_string(),
            "input contains mixed action types: Mixed actions of different types. Original group: \
             'BundlableGeneral', Additional group: 'Sudo', triggering action: 'SudoAddressChange'"
        );
    }
}
