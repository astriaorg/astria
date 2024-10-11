use ibc_types::core::client::Height;

use crate::{
    crypto::VerificationKey,
    primitive::v1::{
        asset::Denom,
        Address,
        RollupId,
    },
    protocol::transaction::v1alpha1::action::{
        group::{
            Actions,
            ErrorKind,
            Group,
        },
        Action,
        BridgeLock,
        BridgeSudoChange,
        BridgeUnlock,
        FeeAssetChange,
        FeeChange,
        FeeChangeKind,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20Withdrawal,
        InitBridgeAccount,
        Sequence,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
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
        Action::Sequence(Sequence {
            rollup_id: RollupId::from([8; 32]),
            data: vec![].into(),
            fee_asset: asset.clone(),
        }),
        Action::Transfer(Transfer {
            to: address,
            amount: 100,
            asset: asset.clone(),
            fee_asset: asset.clone(),
        }),
        Action::BridgeLock(BridgeLock {
            to: address,
            amount: 100,
            asset: asset.clone(),
            fee_asset: asset.clone(),
            destination_chain_address: String::new(),
        }),
        Action::BridgeUnlock(BridgeUnlock {
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
        Group::BundleableGeneral
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
        Action::FeeChange(FeeChange {
            fee_change: FeeChangeKind::TransferBaseFee,
            new_value: 100,
        }),
        Action::FeeAssetChange(FeeAssetChange::Addition(asset)),
        Action::IbcRelayerChange(IbcRelayerChange::Addition(address)),
    ];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        Group::BundleableSudo
    ));
}

#[test]
fn from_list_of_actions_unbundleable_sudo() {
    let address: Address<_> = Address::builder()
        .array([0; 20])
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap();

    let actions = vec![Action::SudoAddressChange(SudoAddressChange {
        new_address: address,
    })];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        Group::UnbundleableSudo
    ));

    let actions = vec![Action::IbcSudoChange(IbcSudoChange {
        new_address: address,
    })];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        Group::UnbundleableSudo
    ));

    let actions = vec![
        Action::SudoAddressChange(SudoAddressChange {
            new_address: address,
        }),
        Action::SudoAddressChange(SudoAddressChange {
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

    let init_bridge_account_action = InitBridgeAccount {
        rollup_id: RollupId::from([8; 32]),
        asset: asset.clone(),
        fee_asset: asset.clone(),
        sudo_address: Some(address),
        withdrawer_address: Some(address),
    };

    let sudo_bridge_address_change_action = BridgeSudoChange {
        new_sudo_address: Some(address),
        bridge_address: address,
        new_withdrawer_address: Some(address),
        fee_asset: asset.clone(),
    };

    let actions = vec![init_bridge_account_action.clone().into()];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        Group::UnbundleableGeneral
    ));

    let actions = vec![sudo_bridge_address_change_action.clone().into()];

    assert!(matches!(
        Actions::try_from_list_of_actions(actions).unwrap().group(),
        Group::UnbundleableGeneral
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
        Action::Sequence(Sequence {
            rollup_id: RollupId::from([8; 32]),
            data: vec![].into(),
            fee_asset: asset.clone(),
        }),
        Action::SudoAddressChange(SudoAddressChange {
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
    assert!(Group::UnbundleableSudo < Group::BundleableSudo);
    assert!(Group::BundleableSudo < Group::UnbundleableGeneral);
    assert!(Group::UnbundleableGeneral < Group::BundleableGeneral);
}
