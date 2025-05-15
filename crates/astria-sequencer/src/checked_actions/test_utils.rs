use astria_core::{
    crypto::VerificationKey,
    primitive::v1::{
        Address,
        ADDRESS_LEN,
    },
    protocol::{
        fees::v1::FeeComponents,
        transaction::v1::{
            action::{
                FeeAssetChange,
                FeeChange,
                IbcRelayerChange,
                IbcSudoChange,
                SudoAddressChange,
                ValidatorUpdate,
            },
            Action,
        },
    },
};

use crate::test_utils::{
    astria_address,
    dummy_bridge_lock,
    dummy_bridge_sudo_change,
    dummy_bridge_transfer,
    dummy_bridge_unlock,
    dummy_currency_pairs_change,
    dummy_ibc_relay,
    dummy_ics20_withdrawal,
    dummy_init_bridge_account,
    dummy_markets_change,
    dummy_recover_ibc_client,
    dummy_rollup_data_submission,
    dummy_transfer,
};

pub(crate) fn dummy_actions() -> [Action; 18] {
    let validator_update = ValidatorUpdate {
        power: 101,
        verification_key: VerificationKey::try_from([10; 32]).unwrap(),
        name: "validator one".parse().unwrap(),
    };
    let sudo_address_change = SudoAddressChange {
        new_address: astria_address(&[2; ADDRESS_LEN]),
    };
    let ibc_sudo_change = IbcSudoChange {
        new_address: astria_address(&[2; ADDRESS_LEN]),
    };
    let ibc_relayer_change = IbcRelayerChange::Addition(astria_address(&[50; ADDRESS_LEN]));
    let fee_asset_change = FeeAssetChange::Addition("test".parse().unwrap());
    let fee_change = FeeChange::Transfer(FeeComponents::new(1, 2));

    [
        Action::RollupDataSubmission(dummy_rollup_data_submission()),
        Action::Transfer(dummy_transfer()),
        Action::ValidatorUpdate(validator_update),
        Action::SudoAddressChange(sudo_address_change),
        Action::Ibc(dummy_ibc_relay()),
        Action::IbcSudoChange(ibc_sudo_change),
        Action::Ics20Withdrawal(dummy_ics20_withdrawal()),
        Action::IbcRelayerChange(ibc_relayer_change),
        Action::FeeAssetChange(fee_asset_change),
        Action::InitBridgeAccount(dummy_init_bridge_account()),
        Action::BridgeLock(dummy_bridge_lock()),
        Action::BridgeUnlock(dummy_bridge_unlock()),
        Action::BridgeSudoChange(dummy_bridge_sudo_change()),
        Action::BridgeTransfer(dummy_bridge_transfer()),
        Action::FeeChange(fee_change),
        Action::RecoverIbcClient(dummy_recover_ibc_client()),
        Action::CurrencyPairsChange(dummy_currency_pairs_change()),
        Action::MarketsChange(dummy_markets_change()),
    ]
}

pub(super) fn address_with_prefix(address_bytes: [u8; ADDRESS_LEN], prefix: &str) -> Address {
    Address::builder()
        .array(address_bytes)
        .prefix(prefix)
        .try_build()
        .unwrap()
}
