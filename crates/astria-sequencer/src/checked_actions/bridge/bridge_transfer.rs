use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::{
        BridgeLock,
        BridgeTransfer,
        BridgeUnlock,
    },
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    instrument,
    Level,
};

use super::{
    super::AssetTransfer,
    bridge_lock::CheckedBridgeLockImpl,
    bridge_unlock::CheckedBridgeUnlockImpl,
};
use crate::accounts::StateWriteExt as _;

#[derive(Debug)]
pub(crate) struct CheckedBridgeTransfer {
    action: BridgeTransfer,
    checked_bridge_unlock: CheckedBridgeUnlockImpl<false>,
    checked_bridge_lock: CheckedBridgeLockImpl<false>,
}

impl CheckedBridgeTransfer {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn new<S: StateRead>(
        action: BridgeTransfer,
        tx_signer: [u8; ADDRESS_LEN],
        tx_id: TransactionId,
        position_in_tx: u64,
        state: S,
    ) -> Result<Self> {
        let BridgeTransfer {
            to,
            amount,
            fee_asset,
            destination_chain_address,
            bridge_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        } = action.clone();

        let bridge_unlock = BridgeUnlock {
            to,
            amount,
            memo: String::new(),
            rollup_withdrawal_event_id,
            rollup_block_number,
            fee_asset: fee_asset.clone(),
            bridge_address,
        };
        let checked_bridge_unlock =
            CheckedBridgeUnlockImpl::<false>::new(bridge_unlock, tx_signer, &state)
                .await
                .wrap_err("failed to construct checked bridge unlock for bridge transfer")?;

        let bridge_lock = BridgeLock {
            to,
            amount,
            asset: checked_bridge_unlock.bridge_account_ibc_asset().into(),
            fee_asset,
            destination_chain_address,
        };
        let checked_bridge_lock = CheckedBridgeLockImpl::<false>::new(
            bridge_lock,
            tx_signer,
            tx_id,
            position_in_tx,
            &state,
        )
        .await
        .wrap_err("failed to construct checked bridge lock for bridge transfer")?;

        let checked_action = Self {
            action,
            checked_bridge_unlock,
            checked_bridge_lock,
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn run_mutable_checks<S: StateRead>(
        &self,
        state: S,
    ) -> Result<()> {
        self.checked_bridge_unlock
            .run_mutable_checks(&state)
            .await
            .wrap_err("mutable checks for bridge unlock failed for bridge transfer")?;
        self.checked_bridge_lock
            .run_mutable_checks(&state)
            .await
            .wrap_err("mutable checks for bridge lock failed for bridge transfer")
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn execute<S: StateWrite>(
        &self,
        mut state: S,
    ) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        let from = &self.checked_bridge_unlock.action().bridge_address;
        let to = &self.checked_bridge_unlock.action().to;
        let asset = self.checked_bridge_unlock.bridge_account_ibc_asset();
        let amount = self.checked_bridge_unlock.action().amount;

        state
            .decrease_balance(from, asset, amount)
            .await
            .wrap_err("failed to decrease bridge account balance")?;
        state
            .increase_balance(to, asset, amount)
            .await
            .wrap_err("failed to increase destination account balance")?;

        self.checked_bridge_lock.record_deposit(&mut state);
        self.checked_bridge_unlock.record_withdrawal_event(state)
    }

    pub(in crate::checked_actions) fn action(&self) -> &BridgeTransfer {
        &self.action
    }
}

impl AssetTransfer for CheckedBridgeTransfer {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        self.checked_bridge_lock.transfer_asset_and_amount()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use astria_core::{
        primitive::v1::{
            asset::{
                Denom,
                IbcPrefixed,
            },
            RollupId,
        },
        protocol::transaction::v1::action::*,
    };

    use super::*;
    use crate::{
        bridge::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        checked_actions::{
            test_utils::address_with_prefix,
            CheckedBridgeSudoChange,
            CheckedBridgeUnlock,
        },
        test_utils::{
            assert_error_contains,
            astria_address,
            dummy_bridge_transfer,
            nria,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_amount_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeTransfer {
            amount: 0,
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "amount must be greater than zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_withdrawal_event_id_empty() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeTransfer {
            rollup_withdrawal_event_id: String::new(),
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup withdrawal event id must be non-empty");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_withdrawal_event_id_too_long() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeTransfer {
            rollup_withdrawal_event_id: ['a'; 257].into_iter().collect(),
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "rollup withdrawal event id must not be more than 256 bytes",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_block_number_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeTransfer {
            rollup_block_number: 0,
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup block number must be greater than zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_destination_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = BridgeTransfer {
            to: address_with_prefix([2; ADDRESS_LEN], prefix),
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = BridgeTransfer {
            bridge_address: address_with_prefix([50; ADDRESS_LEN], prefix),
            ..dummy_bridge_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_account_not_initialized() {
        let fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "failed to get bridge account asset ID; account is not a bridge account",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_authorized() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .with_withdrawer_address([2; 20])
            .init()
            .await;
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "signer is not the authorized withdrawer for the bridge account",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_withdrawal_event_id_already_used() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let rollup_block_number = 999;
        let event_id = action.rollup_withdrawal_event_id.clone();
        fixture
            .state_mut()
            .put_withdrawal_event_rollup_block_number(
                &action.bridge_address,
                &event_id,
                rollup_block_number,
            )
            .unwrap();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "withdrawal event ID `{event_id}` was already executed (rollup block number \
                 {rollup_block_number})"
            ),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_destination_asset_not_same_as_source() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        fixture
            .bridge_initializer(action.to)
            .with_asset(IbcPrefixed::new([10; 32]))
            .init()
            .await;

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "asset ID is not authorized for transfer to bridge account",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_asset_mapping_fails() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        let asset = IbcPrefixed::new([10; 32]);
        fixture
            .bridge_initializer(action.bridge_address)
            .with_asset(asset)
            .init()
            .await;
        fixture
            .bridge_initializer(action.to)
            .with_asset(asset)
            .init()
            .await;

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "mapping from IBC prefixed bridge asset to trace prefixed not found",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_authorized() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked bridge transfer while the tx signer is the authorized withdrawer.
        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        fixture.bridge_initializer(action.to).init().await;
        let checked_action: CheckedBridgeTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Change the withdrawer address.
        let bridge_sudo_change = BridgeSudoChange {
            bridge_address: action.bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: Some(astria_address(&[2; 20])),
            fee_asset: nria().into(),
            disable_deposits: false,
        };
        let checked_bridge_sudo_change: CheckedBridgeSudoChange = fixture
            .new_checked_action(bridge_sudo_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_bridge_sudo_change
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked bridge transfer now - should fail due to tx signer no longer
        // being the authorized withdrawer.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "signer is not the authorized withdrawer for the bridge account",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_withdrawal_event_id_already_used() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked bridge transfer while the withdrawal event ID is unused.
        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        fixture.bridge_initializer(action.to).init().await;
        let checked_action: CheckedBridgeTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute a bridge unlock with the same withdrawal event ID.
        let bridge_unlock = BridgeUnlock {
            to: astria_address(&[3; ADDRESS_LEN]),
            amount: 1,
            fee_asset: nria().into(),
            bridge_address: action.bridge_address,
            memo: "a".to_string(),
            rollup_block_number: 8,
            rollup_withdrawal_event_id: action.rollup_withdrawal_event_id.clone(),
        };
        let checked_bridge_unlock: CheckedBridgeUnlock = fixture
            .new_checked_action(bridge_unlock.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Provide the bridge account with sufficient balance to execute the bridge unlock.
        fixture
            .state_mut()
            .increase_balance(&bridge_unlock.bridge_address, &nria(), bridge_unlock.amount)
            .await
            .unwrap();
        checked_bridge_unlock
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked bridge transfer now with the same withdrawal event ID - should
        // fail due to the ID being used already.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "withdrawal event ID `{}` was already executed (rollup block number {})",
                action.rollup_withdrawal_event_id, bridge_unlock.rollup_block_number
            ),
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_bridge_account_has_insufficient_balance() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_transfer();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        fixture.bridge_initializer(action.to).init().await;
        let checked_action: CheckedBridgeTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "failed to decrease bridge account balance");
    }

    #[tokio::test]
    async fn should_fail_if_bridge_deposits_disabled() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked bridge transfer while the account has insufficient balance to
        // ensure balance checks are only part of execution.
        let action = dummy_bridge_transfer();
        let rollup_id = RollupId::new([7; 32]);
        let asset = nria();
        fixture
            .bridge_initializer(action.bridge_address)
            .with_asset(asset.clone())
            .init()
            .await;
        fixture
            .bridge_initializer(action.to)
            .with_rollup_id(rollup_id)
            .init()
            .await;
        let checked_action: CheckedBridgeTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Provide the bridge account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&action.bridge_address, &nria(), action.amount)
            .await
            .unwrap();

        // Disable Deposits of the into transfer bridge.
        let bridge_sudo_change = BridgeSudoChange {
            bridge_address: action.to,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: nria().into(),
            disable_deposits: true,
        };
        let checked_bridge_sudo_change: CheckedBridgeSudoChange = fixture
            .new_checked_action(bridge_sudo_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_bridge_sudo_change
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Check the balances are correct before execution.
        assert_eq!(
            fixture.get_nria_balance(&action.bridge_address).await,
            action.amount
        );
        assert_eq!(fixture.get_nria_balance(&action.to).await, 0);

        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "bridge account deposits are currently disabled");
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked bridge transfer while the account has insufficient balance to
        // ensure balance checks are only part of execution.
        let action = dummy_bridge_transfer();
        let rollup_id = RollupId::new([7; 32]);
        let asset = nria();
        fixture
            .bridge_initializer(action.bridge_address)
            .with_asset(asset.clone())
            .init()
            .await;
        fixture
            .bridge_initializer(action.to)
            .with_rollup_id(rollup_id)
            .init()
            .await;
        let checked_action: CheckedBridgeTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Provide the bridge account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&action.bridge_address, &nria(), action.amount)
            .await
            .unwrap();

        // Check the balances are correct before execution.
        assert_eq!(
            fixture.get_nria_balance(&action.bridge_address).await,
            action.amount
        );
        assert_eq!(fixture.get_nria_balance(&action.to).await, 0);

        checked_action.execute(fixture.state_mut()).await.unwrap();

        // Check the balances are correct after execution.
        assert_eq!(fixture.get_nria_balance(&action.bridge_address).await, 0);
        assert_eq!(fixture.get_nria_balance(&action.to).await, action.amount);

        // Check the rollup block number is recorded under the given event ID.
        let rollup_block_number = fixture
            .state()
            .get_withdrawal_event_rollup_block_number(
                &action.bridge_address,
                &action.rollup_withdrawal_event_id,
            )
            .await
            .unwrap();
        assert_eq!(rollup_block_number, Some(action.rollup_block_number));

        // Check the deposit is recorded.
        let deposits = fixture
            .state()
            .get_cached_block_deposits()
            .get(&rollup_id)
            .unwrap()
            .clone();
        assert_eq!(deposits.len(), 1);
        let deposit = &deposits[0];
        assert_eq!(deposit.bridge_address, action.to);
        assert_eq!(deposit.rollup_id, rollup_id);
        assert_eq!(deposit.amount, action.amount);
        assert_eq!(deposit.asset, Denom::from(asset));
        assert_eq!(
            deposit.destination_chain_address,
            action.destination_chain_address
        );
        assert_eq!(deposit.source_transaction_id.get(), [11; 32]);
        assert_eq!(deposit.source_action_index, 11);

        // Check the deposit event is cached.
        let deposit_events = fixture.into_events();
        assert_eq!(deposit_events.len(), 1);
        assert_eq!(deposit_events[0].kind, "tx.deposit");
    }
}
