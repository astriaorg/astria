use astria_core::{
    primitive::v1::RollupId,
    protocol::transaction::v1::action,
};
use cnidarium::StateDelta;
use ibc_types::core::client::Height;

use crate::{
    action_handler::{
        actions::establish_withdrawal_target,
        tests::test_asset,
    },
    address::StateWriteExt as _,
    bridge::StateWriteExt as _,
    test_utils::{
        assert_eyre_error,
        astria_address,
        ASTRIA_PREFIX,
    },
};

#[tokio::test]
async fn withdrawal_target_is_sender_if_bridge_is_not_set_and_sender_is_not_bridge() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let state = StateDelta::new(snapshot);

    let denom = test_asset();
    let from = [1u8; 20];
    let action = action::Ics20Withdrawal {
        amount: 1,
        denom: denom.clone(),
        bridge_address: None,
        destination_chain_address: "test".to_string(),
        return_address: astria_address(&from),
        timeout_height: Height::new(1, 1).unwrap(),
        timeout_time: 1,
        source_channel: "channel-0".to_string().parse().unwrap(),
        fee_asset: denom.clone(),
        memo: String::new(),
        use_compat_address: false,
    };

    assert_eq!(
        *establish_withdrawal_target(&action, &state, &from)
            .await
            .unwrap(),
        from
    );
}

#[tokio::test]
async fn withdrawal_target_is_sender_if_bridge_is_unset_but_sender_is_bridge() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    // sender is a bridge address, which is also the withdrawer, so it's ok
    let bridge_address = [1u8; 20];
    state
        .put_bridge_account_rollup_id(
            &bridge_address,
            RollupId::from_unhashed_bytes("testrollupid"),
        )
        .unwrap();
    state
        .put_bridge_account_withdrawer_address(&bridge_address, bridge_address)
        .unwrap();

    let denom = test_asset();
    let action = action::Ics20Withdrawal {
        amount: 1,
        denom: denom.clone(),
        bridge_address: None,
        destination_chain_address: "test".to_string(),
        return_address: astria_address(&bridge_address),
        timeout_height: Height::new(1, 1).unwrap(),
        timeout_time: 1,
        source_channel: "channel-0".to_string().parse().unwrap(),
        fee_asset: denom.clone(),
        memo: String::new(),
        use_compat_address: false,
    };

    assert_eyre_error(
        &establish_withdrawal_target(&action, &state, &bridge_address)
            .await
            .unwrap_err(),
        "sender cannot be a bridge address if bridge address is not set",
    );
}

mod bridge_sender_is_rejected_because_it_is_not_a_withdrawer {
    use super::*;

    fn bridge_address() -> [u8; 20] {
        [1; 20]
    }

    fn action() -> action::Ics20Withdrawal {
        action::Ics20Withdrawal {
            amount: 1,
            denom: test_asset(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: astria_address(&[1; 20]),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: test_asset(),
            memo: String::new(),
            use_compat_address: false,
        }
    }

    async fn run_test(action: action::Ics20Withdrawal) {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // withdraw is *not* the bridge address, Ics20Withdrawal must be sent by the withdrawer
        state
            .put_bridge_account_rollup_id(
                &bridge_address(),
                RollupId::from_unhashed_bytes("testrollupid"),
            )
            .unwrap();
        state
            .put_bridge_account_withdrawer_address(&bridge_address(), astria_address(&[2u8; 20]))
            .unwrap();

        assert_eyre_error(
            &establish_withdrawal_target(&action, &state, &bridge_address())
                .await
                .unwrap_err(),
            "sender does not match bridge withdrawer address; unauthorized",
        );
    }

    #[tokio::test]
    async fn bridge_set() {
        let mut action = action();
        action.bridge_address = Some(astria_address(&bridge_address()));
        run_test(action).await;
    }
}

#[tokio::test]
async fn bridge_sender_is_withdrawal_target() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state = StateDelta::new(snapshot);

    state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

    // sender the withdrawer address, so it's ok
    let bridge_address = [1u8; 20];
    let withdrawer_address = [2u8; 20];
    state
        .put_bridge_account_rollup_id(
            &bridge_address,
            RollupId::from_unhashed_bytes("testrollupid"),
        )
        .unwrap();
    state
        .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
        .unwrap();

    let denom = test_asset();
    let action = action::Ics20Withdrawal {
        amount: 1,
        denom: denom.clone(),
        bridge_address: Some(astria_address(&bridge_address)),
        destination_chain_address: "test".to_string(),
        return_address: astria_address(&bridge_address),
        timeout_height: Height::new(1, 1).unwrap(),
        timeout_time: 1,
        source_channel: "channel-0".to_string().parse().unwrap(),
        fee_asset: denom.clone(),
        memo: String::new(),
        use_compat_address: false,
    };

    assert_eq!(
        *establish_withdrawal_target(&action, &state, &withdrawer_address)
            .await
            .unwrap(),
        bridge_address,
    );
}

#[tokio::test]
async fn bridge_is_rejected_as_withdrawal_target_because_it_has_no_withdrawer_address_set() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let state = StateDelta::new(snapshot);

    // sender is not the withdrawer address, so must fail
    let not_bridge_address = [1u8; 20];

    let denom = test_asset();
    let action = action::Ics20Withdrawal {
        amount: 1,
        denom: denom.clone(),
        bridge_address: Some(astria_address(&not_bridge_address)),
        destination_chain_address: "test".to_string(),
        return_address: astria_address(&not_bridge_address),
        timeout_height: Height::new(1, 1).unwrap(),
        timeout_time: 1,
        source_channel: "channel-0".to_string().parse().unwrap(),
        fee_asset: denom.clone(),
        memo: String::new(),
        use_compat_address: false,
    };

    assert_eyre_error(
        &establish_withdrawal_target(&action, &state, &not_bridge_address)
            .await
            .unwrap_err(),
        "bridge address must have a withdrawer address set",
    );
}
