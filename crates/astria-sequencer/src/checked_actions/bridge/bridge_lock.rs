use astria_core::{
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
        },
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::BridgeLock,
    sequencerblock::v1::block::Deposit,
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
    assets::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    checked_actions::{
        AssetTransfer,
        TransactionSignerAddressBytes,
    },
    utils::create_deposit_event,
};

pub(crate) type CheckedBridgeLock = CheckedBridgeLockImpl<true>;

/// This struct provides the implementation details of the checked bridge lock action.
///
/// It is also used to perform checks on a bridge transfer action, which is essentially a cross
/// between a bridge unlock and a bridge lock.
///
/// A `BridgeLock` action uses the tx signer as the source account, and does not allow the tx signer
/// to be a bridge account, whereas `BridgeTransfer` provides a `bridge_account` as the source where
/// this may not be tx signer's account, and is required to be a bridge account. Hence a bridge lock
/// is implemented via `CheckedBridgeLockImpl<true>` and has methods to allow checking AND executing
/// the action, while the checks relevant to a bridge transfer are implemented via
/// `CheckedBridgeLockImpl<false>`, where this has no method supporting execution.
#[derive(Debug)]
pub(crate) struct CheckedBridgeLockImpl<const PURE_LOCK: bool> {
    action: BridgeLock,
    tx_signer: TransactionSignerAddressBytes,
    /// The deposit created from this action.
    deposit: Deposit,
}

impl<const PURE_LOCK: bool> CheckedBridgeLockImpl<PURE_LOCK> {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn new<S: StateRead>(
        action: BridgeLock,
        tx_signer: [u8; ADDRESS_LEN],
        tx_id: TransactionId,
        position_in_tx: u64,
        state: S,
    ) -> Result<Self> {
        state
            .ensure_base_prefix(&action.to)
            .await
            .wrap_err("destination address has an unsupported prefix")?;

        // check that the asset to be transferred matches the bridge account asset.
        // this also implicitly ensures the recipient is a bridge account.
        let allowed_asset = state
            .get_bridge_account_ibc_asset(&action.to)
            .await
            .wrap_err(
                "failed to get bridge account asset ID; destination account is not a bridge \
                 account",
            )?;
        ensure!(
            allowed_asset == action.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        // Try to construct the `Deposit`.
        let rollup_id = state
            .get_bridge_account_rollup_id(&action.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;
        // Map asset to trace prefixed asset for deposit, if it is not already. The IBC asset cannot
        // be changed once set in state, so if `map_ibc_to_trace_prefixed_asset` succeeds now it
        // can't fail later during execution.
        let deposit_asset = match &action.asset {
            Denom::TracePrefixed(asset) => asset.clone(),
            Denom::IbcPrefixed(asset) => state
                .map_ibc_to_trace_prefixed_asset(asset)
                .await
                .wrap_err("failed to map IBC asset to trace prefixed asset")?
                .ok_or_eyre("mapping from IBC prefixed bridge asset to trace prefixed not found")?,
        };
        let deposit = Deposit {
            bridge_address: action.to,
            rollup_id,
            amount: action.amount,
            asset: deposit_asset.into(),
            destination_chain_address: action.destination_chain_address.clone(),
            source_transaction_id: tx_id,
            source_action_index: position_in_tx,
        };

        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
            deposit,
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn run_mutable_checks<S: StateRead>(
        &self,
        state: S,
    ) -> Result<()> {
        if PURE_LOCK
            && state
                .is_a_bridge_account(&self.tx_signer)
                .await
                .wrap_err("failed to check if signer is a bridge account")?
        {
            bail!("bridge accounts cannot send bridge locks");
        }

        let bridge_disabled = state
            .is_bridge_account_disabled(&self.action.to)
            .await
            .wrap_err("failed to read whether bridge account deposits are disabled from storage")?;
        ensure!(
            !bridge_disabled,
            "bridge account deposits are currently disabled"
        );

        Ok(())
    }

    pub(in crate::checked_actions) fn action(&self) -> &BridgeLock {
        &self.action
    }

    pub(super) fn record_deposit<S: StateWrite>(&self, mut state: S) {
        let deposit_abci_event = create_deposit_event(&self.deposit);
        state.cache_deposit_event(self.deposit.clone());
        state.record(deposit_abci_event);
    }
}

impl CheckedBridgeLockImpl<true> {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(in crate::checked_actions) async fn execute<S: StateWrite>(
        &self,
        mut state: S,
    ) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        state
            .decrease_balance(&self.tx_signer, &self.action.asset, self.action.amount)
            .await
            .wrap_err("failed to decrease signer account balance")?;
        state
            .increase_balance(&self.action.to, &self.action.asset, self.action.amount)
            .await
            .wrap_err("failed to increase destination account balance")?;

        self.record_deposit(state);

        Ok(())
    }
}

impl<const PURE_LOCK: bool> AssetTransfer for CheckedBridgeLockImpl<PURE_LOCK> {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        Some((self.action.asset.to_ibc_prefixed(), self.action.amount))
    }
}

// NOTE: unit tests here cover only `CheckedBridgeLockImpl<true>`.  Test coverage of
// `CheckedBridgeLockImpl<false>` is in `checked_actions::bridge_transfer`.
#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::ADDRESS_LENGTH,
        primitive::v1::{
            asset::{
                IbcPrefixed,
                TracePrefixed,
            },
            RollupId,
        },
        protocol::transaction::v1::action::*,
    };

    use super::*;
    use crate::{
        assets::StateWriteExt as _,
        checked_actions::{
            test_utils::address_with_prefix,
            CheckedBridgeSudoChange,
            CheckedInitBridgeAccount,
        },
        test_utils::{
            assert_error_contains,
            astria_address,
            dummy_bridge_lock,
            nria,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_destination_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = BridgeLock {
            to: address_with_prefix([50; ADDRESS_LEN], prefix),
            ..dummy_bridge_lock()
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
    async fn should_fail_construction_if_destination_asset_not_allowed() {
        let mut fixture = Fixture::default_initialized().await;

        let action = BridgeLock {
            asset: Denom::IbcPrefixed(IbcPrefixed::new([10; 32])),
            ..dummy_bridge_lock()
        };
        fixture.bridge_initializer(action.to).init().await;
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

        let asset = Denom::IbcPrefixed(IbcPrefixed::new([10; 32]));
        let action = BridgeLock {
            asset: asset.clone(),
            ..dummy_bridge_lock()
        };
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
    async fn should_fail_construction_if_signer_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_lock();
        fixture.bridge_initializer(action.to).init().await;
        let tx_signer = [2; ADDRESS_LENGTH];
        fixture
            .bridge_initializer(astria_address(&tx_signer))
            .init()
            .await;

        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();

        assert_error_contains(&err, "bridge accounts cannot send bridge locks");
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked bridge lock while the signer account is not a bridge account.
        let action = dummy_bridge_lock();
        fixture.bridge_initializer(action.to).init().await;
        let tx_signer = [2; ADDRESS_LENGTH];
        let checked_action: CheckedBridgeLock = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap()
            .into();

        // Initialize the signer's account as a bridge account.
        let init_bridge_account = InitBridgeAccount {
            rollup_id: RollupId::new([2; 32]),
            asset: "test".parse().unwrap(),
            fee_asset: "test".parse().unwrap(),
            sudo_address: None,
            withdrawer_address: None,
        };
        let checked_init_bridge_account: CheckedInitBridgeAccount = fixture
            .new_checked_action(init_bridge_account, tx_signer)
            .await
            .unwrap()
            .into();
        checked_init_bridge_account
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked bridge lock now - should fail due to bridge account now
        // existing.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "bridge accounts cannot send bridge locks");
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_account_has_insufficient_balance() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_lock();
        fixture.bridge_initializer(action.to).init().await;
        let tx_signer = [2; ADDRESS_LENGTH];
        let checked_action: CheckedBridgeLock = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap()
            .into();

        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "failed to decrease signer account balance");
    }

    #[tokio::test]
    async fn should_fail_when_bridge_desposits_disabled() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked bridge lock while the account has insufficient balance to ensure
        // balance checks are only part of execution.
        let action = dummy_bridge_lock();
        let rollup_id = RollupId::new([10; 32]);
        fixture
            .bridge_initializer(action.to)
            .with_rollup_id(rollup_id)
            .init()
            .await;
        let tx_signer = [2; ADDRESS_LENGTH];
        let checked_action: CheckedBridgeLock = fixture
            .new_checked_action(action.clone(), tx_signer)
            .await
            .unwrap()
            .into();

        // Provide the signer account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&tx_signer, &action.asset, action.amount)
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

        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "bridge account deposits are currently disabled");
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked bridge lock while the account has insufficient balance to ensure
        // balance checks are only part of execution.
        let action = dummy_bridge_lock();
        let rollup_id = RollupId::new([10; 32]);
        fixture
            .bridge_initializer(action.to)
            .with_rollup_id(rollup_id)
            .init()
            .await;
        let tx_signer = [2; ADDRESS_LENGTH];
        let checked_action: CheckedBridgeLock = fixture
            .new_checked_action(action.clone(), tx_signer)
            .await
            .unwrap()
            .into();

        // Provide the signer account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&tx_signer, &action.asset, action.amount)
            .await
            .unwrap();

        // Check the balances are correct before execution.
        assert_eq!(fixture.get_nria_balance(&tx_signer).await, action.amount);
        assert_eq!(fixture.get_nria_balance(&action.to).await, 0);

        checked_action.execute(fixture.state_mut()).await.unwrap();

        // Check the balances are correct after execution.
        assert_eq!(fixture.get_nria_balance(&tx_signer).await, 0);
        assert_eq!(fixture.get_nria_balance(&action.to).await, action.amount);

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
        assert_eq!(deposit.asset, action.asset);
        assert_eq!(
            deposit.destination_chain_address,
            action.destination_chain_address
        );
        assert_eq!(deposit.source_transaction_id.get(), [10; 32]);
        assert_eq!(deposit.source_action_index, 10);

        // Check the deposit event is cached.
        let deposit_events = fixture.into_events();
        assert_eq!(deposit_events.len(), 1);
        assert_eq!(deposit_events[0].kind, "tx.deposit");
    }

    #[tokio::test]
    async fn should_map_ibc_to_trace_prefixed_for_deposit() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the bridge lock with an IBC denom, and check it is recorded in the `Deposit` as
        // trace-prefixed.
        let trace_asset = "trace_asset".parse::<TracePrefixed>().unwrap();
        let ibc_asset = trace_asset.to_ibc_prefixed();
        let action = BridgeLock {
            asset: Denom::IbcPrefixed(ibc_asset),
            ..dummy_bridge_lock()
        };

        let rollup_id = RollupId::new([10; 32]);
        fixture
            .bridge_initializer(action.to)
            .with_rollup_id(rollup_id)
            .with_asset(ibc_asset)
            .init()
            .await;
        fixture
            .state_mut()
            .put_ibc_asset(trace_asset.clone())
            .unwrap();
        let tx_signer = [2; ADDRESS_LENGTH];
        let checked_action: CheckedBridgeLock = fixture
            .new_checked_action(action.clone(), tx_signer)
            .await
            .unwrap()
            .into();

        fixture
            .state_mut()
            .increase_balance(&tx_signer, &action.asset, action.amount)
            .await
            .unwrap();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        let deposits = &fixture
            .state()
            .get_cached_block_deposits()
            .get(&rollup_id)
            .unwrap()
            .clone();
        assert!(deposits[0].asset.as_trace_prefixed().is_some());
    }
}
