use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::BridgeUnlock,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    OptionExt as _,
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

use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    checked_actions::{
        AssetTransfer,
        TransactionSignerAddressBytes,
    },
};

pub(crate) type CheckedBridgeUnlock = CheckedBridgeUnlockImpl<true>;

/// This struct provides the implementation details of the checked bridge unlock action.
///
/// It is also used to perform checks on a bridge transfer action, which is essentially a cross
/// between a bridge unlock and a bridge lock.
///
/// A `BridgeUnlock` action does not allow unlocking to a bridge account, whereas `BridgeTransfer`
/// requires the `to` account to be a bridge one. Hence a bridge unlock is implemented via
/// `CheckedBridgeUnlockImpl<true>` and has methods to allow checking AND executing the action,
/// while the checks relevant to a bridge transfer are implemented via
/// `CheckedBridgeUnlockImpl<false>`, where this has no method supporting execution.
#[derive(Debug)]
pub(crate) struct CheckedBridgeUnlockImpl<const PURE_UNLOCK: bool> {
    action: BridgeUnlock,
    tx_signer: TransactionSignerAddressBytes,
    bridge_account_ibc_asset: IbcPrefixed,
}

impl<const PURE_UNLOCK: bool> CheckedBridgeUnlockImpl<PURE_UNLOCK> {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn new<S: StateRead>(
        action: BridgeUnlock,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        // TODO(https://github.com/astriaorg/astria/issues/1430): move stateless checks to the
        // `BridgeUnlock` parsing.
        ensure!(action.amount > 0, "amount must be greater than zero");
        ensure!(
            action.memo.len() <= 64,
            "memo must not be more than 64 bytes"
        );
        ensure!(
            !action.rollup_withdrawal_event_id.is_empty(),
            "rollup withdrawal event id must be non-empty",
        );
        ensure!(
            action.rollup_withdrawal_event_id.len() <= 256,
            "rollup withdrawal event id must not be more than 256 bytes",
        );
        ensure!(
            action.rollup_block_number > 0,
            "rollup block number must be greater than zero",
        );

        state
            .ensure_base_prefix(&action.to)
            .await
            .wrap_err("destination address has an unsupported prefix")?;
        state
            .ensure_base_prefix(&action.bridge_address)
            .await
            .wrap_err("source address has an unsupported prefix")?;

        let bridge_account_ibc_asset = state
            .get_bridge_account_ibc_asset(&action.bridge_address)
            .await
            .wrap_err("failed to get bridge account asset ID; account is not a bridge account")?;

        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
            bridge_account_ibc_asset,
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn run_mutable_checks<S: StateRead>(
        &self,
        state: S,
    ) -> Result<()> {
        if PURE_UNLOCK
            && state
                .is_a_bridge_account(&self.action.to)
                .await
                .wrap_err("failed to check if `to` address is a bridge account")?
        {
            bail!("bridge accounts cannot receive bridge unlocks");
        }

        let withdrawer = state
            .get_bridge_account_withdrawer_address(&self.action.bridge_address)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
            .ok_or_eyre("bridge account must have a withdrawer address set")?;
        ensure!(
            *self.tx_signer.as_bytes() == withdrawer,
            "signer is not the authorized withdrawer for the bridge account",
        );

        if let Some(existing_block_num) = state
            .get_withdrawal_event_rollup_block_number(
                &self.action.bridge_address,
                &self.action.rollup_withdrawal_event_id,
            )
            .await
            .wrap_err("failed to read withdrawal event block number from storage")?
        {
            bail!(
                "withdrawal event ID `{}` was already executed (rollup block number \
                 {existing_block_num})",
                self.action.rollup_withdrawal_event_id
            );
        }

        Ok(())
    }

    pub(in crate::checked_actions) fn action(&self) -> &BridgeUnlock {
        &self.action
    }

    pub(super) fn record_withdrawal_event<S: StateWrite>(&self, mut state: S) -> Result<()> {
        state
            .put_withdrawal_event_rollup_block_number(
                &self.action.bridge_address,
                &self.action.rollup_withdrawal_event_id,
                self.action.rollup_block_number,
            )
            .wrap_err("failed to write withdrawal event block number to storage")
    }
}

impl CheckedBridgeUnlockImpl<true> {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn execute<S: StateWrite>(
        &self,
        mut state: S,
    ) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        state
            .decrease_balance(
                &self.action.bridge_address,
                &self.bridge_account_ibc_asset,
                self.action.amount,
            )
            .await
            .wrap_err("failed to decrease bridge account balance")?;
        state
            .increase_balance(
                &self.action.to,
                &self.bridge_account_ibc_asset,
                self.action.amount,
            )
            .await
            .wrap_err("failed to increase destination account balance")?;

        self.record_withdrawal_event(state)
    }
}

impl CheckedBridgeUnlockImpl<false> {
    pub(super) fn bridge_account_ibc_asset(&self) -> &IbcPrefixed {
        &self.bridge_account_ibc_asset
    }
}

impl AssetTransfer for CheckedBridgeUnlock {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        Some((self.bridge_account_ibc_asset, self.action.amount))
    }
}

// NOTE: unit tests here cover only `CheckedBridgeUnlockImpl<true>`.  Test coverage of
// `CheckedBridgeUnlockImpl<false>` is in `checked_actions::bridge_transfer`.
#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::RollupId,
        protocol::transaction::v1::action::*,
    };

    use super::*;
    use crate::{
        checked_actions::{
            test_utils::address_with_prefix,
            CheckedBridgeSudoChange,
            CheckedInitBridgeAccount,
        },
        test_utils::{
            assert_error_contains,
            astria_address,
            dummy_bridge_unlock,
            nria,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_amount_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeUnlock {
            amount: 0,
            ..dummy_bridge_unlock()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "amount must be greater than zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_memo_too_long() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeUnlock {
            memo: ['a'; 65].into_iter().collect(),
            ..dummy_bridge_unlock()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "memo must not be more than 64 bytes");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_withdrawal_event_id_empty() {
        let fixture = Fixture::default_initialized().await;

        let action = BridgeUnlock {
            rollup_withdrawal_event_id: String::new(),
            ..dummy_bridge_unlock()
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

        let action = BridgeUnlock {
            rollup_withdrawal_event_id: ['a'; 257].into_iter().collect(),
            ..dummy_bridge_unlock()
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

        let action = BridgeUnlock {
            rollup_block_number: 0,
            ..dummy_bridge_unlock()
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
        let action = BridgeUnlock {
            to: address_with_prefix([2; ADDRESS_LEN], prefix),
            ..dummy_bridge_unlock()
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
        let action = BridgeUnlock {
            bridge_address: address_with_prefix([50; ADDRESS_LEN], prefix),
            ..dummy_bridge_unlock()
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

        let action = dummy_bridge_unlock();
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
    async fn should_fail_construction_if_to_address_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_unlock();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        fixture.bridge_initializer(action.to).init().await;
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "bridge accounts cannot receive bridge unlocks");
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_authorized() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_unlock();
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

        let action = dummy_bridge_unlock();
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
    async fn should_fail_execution_if_to_address_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked bridge unlock while the `to` account is not a bridge account.
        let action = dummy_bridge_unlock();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let to_address = action.to.bytes();
        let checked_action: CheckedBridgeUnlock = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Initialize the `to` account as a bridge account.
        let init_bridge_account = InitBridgeAccount {
            rollup_id: RollupId::new([2; 32]),
            asset: "test".parse().unwrap(),
            fee_asset: "test".parse().unwrap(),
            sudo_address: None,
            withdrawer_address: None,
        };
        let checked_init_bridge_account: CheckedInitBridgeAccount = fixture
            .new_checked_action(init_bridge_account, to_address)
            .await
            .unwrap()
            .into();
        checked_init_bridge_account
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked bridge unlock now - should fail due to `to` account now
        // existing.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "bridge accounts cannot receive bridge unlocks");
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_authorized() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked bridge unlock while the tx signer is the authorized withdrawer.
        let action = dummy_bridge_unlock();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let checked_action: CheckedBridgeUnlock = fixture
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

        // Try to execute the checked bridge unlock now - should fail due to tx signer no longer
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

        // Construct two checked bridge unlocks while the withdrawal event ID has not been used.
        let action_1 = dummy_bridge_unlock();
        let event_id = action_1.rollup_withdrawal_event_id.clone();
        fixture
            .bridge_initializer(action_1.bridge_address)
            .init()
            .await;
        let checked_action_1: CheckedBridgeUnlock = fixture
            .new_checked_action(action_1.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let action_2 = BridgeUnlock {
            rollup_block_number: action_1.rollup_block_number.checked_add(1).unwrap(),
            ..action_1.clone()
        };
        let checked_action_2: CheckedBridgeUnlock = fixture
            .new_checked_action(action_2.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute the first bridge unlock to write the withdrawal event ID to state. Need to
        // provide the bridge account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&action_1.bridge_address, &nria(), action_1.amount)
            .await
            .unwrap();
        checked_action_1.execute(fixture.state_mut()).await.unwrap();

        // Try to execute the second checked bridge unlock now with the same withdrawal event ID -
        // should fail due to the ID being used already.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "withdrawal event ID `{event_id}` was already executed (rollup block number {})",
                action_1.rollup_block_number
            ),
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_bridge_account_has_insufficient_balance() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_unlock();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let checked_action: CheckedBridgeUnlock = fixture
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
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked bridge unlock while the account has insufficient balance to ensure
        // balance checks are only part of execution.
        let action = dummy_bridge_unlock();
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let checked_action: CheckedBridgeUnlock = fixture
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
    }
}
