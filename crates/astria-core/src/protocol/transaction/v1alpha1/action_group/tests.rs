use ibc_types::core::client::Height;

use crate::{
    crypto::VerificationKey,
    primitive::v1::{
        asset::Denom,
        Address,
        RollupId,
    },
    protocol::transaction::v1alpha1::{
        action::{
            Action,
            BridgeLockAction,
            BridgeSudoChangeAction,
            BridgeUnlockAction,
            FeeAssetChangeAction,
            FeeChange,
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
        action_group::{
            ActionGroup,
            Actions,
            ErrorKind,
        },
    },
};
const ASTRIA_ADDRESS_PREFIX: &str = "astria";

#[test]
fn try_from_list_of_actions_bundleable_general() {
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
        }),
        Action::Transfer(TransferAction {
            to: address,
            amount: 100,
            asset: asset.clone(),
            fee_asset: asset.clone(),
        }),
        Action::BridgeLock(BridgeLockAction {
            to: address,
            amount: 100,
            asset: asset.clone(),
            fee_asset: asset.clone(),
            destination_chain_address: String::new(),
        }),
        Action::BridgeUnlock(BridgeUnlockAction {
            to: address,
            amount: 100,
            fee_asset: asset.clone(),
            bridge_address: address,
            memo: String::new(),
            rollup_block_number: 0,
            rollup_withdrawal_event_id: String::new(),
        }),
        Action::ValidatorUpdate(ValidatorUpdate {
            power: 100,
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
        }),
        Action::Ics20Withdrawal(Ics20Withdrawal {
            denom: asset.clone(),
            destination_chain_address: String::new(),
            return_address: address,
            amount: 1_000_000u128,
            memo: String::new(),
            fee_asset: asset.clone(),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 0,
            source_channel: "channel-0".parse().unwrap(),
            bridge_address: Some(address),
            use_compat_address: false,
        }),
    ];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::BundleableGeneral
    ));
}

#[test]
fn from_list_of_actions_bundleable_sudo() {
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
        }),
        Action::FeeAssetChange(FeeAssetChangeAction::Addition(asset)),
        Action::IbcRelayerChange(IbcRelayerChangeAction::Addition(address)),
    ];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::BundleableSudo
    ));
}

#[test]
fn from_list_of_actions_unbundleable_sudo() {
    let address: Address<_> = Address::builder()
        .array([0; 20])
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap();

    let actions = vec![Action::SudoAddressChange(SudoAddressChangeAction {
        new_address: address,
    })];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::UnbundleableSudo
    ));

    let actions = vec![Action::IbcSudoChange(IbcSudoChangeAction {
        new_address: address,
    })];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::UnbundleableSudo
    ));

    let actions = vec![
        Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: address,
        }),
        Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: address,
        }),
    ];

    let error_kind = Actions::try_from_list_of_actions(actions).unwrap_err().0;
    assert!(
        matches!(error_kind, ErrorKind::NotBundleable { .. }),
        "expected ErrorKind::NotBundleable, got {error_kind:?}"
    );
}

#[test]
fn from_list_of_actions_unbundleable_general() {
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
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::UnbundleableGeneral
    ));

    let actions = vec![sudo_bridge_address_change_action.clone().into()];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        ActionGroup::UnbundleableGeneral
    ));

    let actions = vec![
        init_bridge_account_action.into(),
        sudo_bridge_address_change_action.into(),
    ];

    let error_kind = Actions::try_from_list_of_actions(actions).unwrap_err().0;
    assert!(
        matches!(error_kind, ErrorKind::NotBundleable { .. }),
        "expected ErrorKind::NotBundleable, got {error_kind:?}"
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
        }),
        Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: address,
        }),
    ];

    let error_kind = Actions::try_from_list_of_actions(actions).unwrap_err().0;
    assert!(
        matches!(error_kind, ErrorKind::Mixed { .. }),
        "expected ErrorKind::Mixed, got {error_kind:?}"
    );
}

#[test]
fn from_list_of_actions_empty() {
    let error_kind = Actions::try_from_list_of_actions(vec![]).unwrap_err().0;
    assert!(
        matches!(error_kind, ErrorKind::Empty { .. }),
        "expected ErrorKind::Empty, got {error_kind:?}"
    );
}

#[test]
fn should_be_in_expected_order() {
    assert!(ActionGroup::UnbundleableSudo < ActionGroup::BundleableSudo);
    assert!(ActionGroup::BundleableSudo < ActionGroup::UnbundleableGeneral);
    assert!(ActionGroup::UnbundleableGeneral < ActionGroup::BundleableGeneral);
}
