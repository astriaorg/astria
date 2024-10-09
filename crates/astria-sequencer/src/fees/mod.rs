use astria_core::{
    primitive::v1::{
        asset,
        TransactionId,
        TRANSACTION_ID_LEN,
    },
    protocol::transaction::v1alpha1::action::{
        self,
        BridgeLock,
        BridgeLockFeeComponents,
        BridgeSudoChange,
        BridgeSudoChangeFeeComponents,
        BridgeUnlock,
        BridgeUnlockFeeComponents,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20WithdrawalFeeComponents,
        InitBridgeAccount,
        InitBridgeAccountFeeComponents,
        Sequence,
        SequenceFeeComponents,
        SudoAddressChange,
        Transfer,
        TransferFeeComponents,
        ValidatorUpdate,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    OptionExt as _,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tendermint::abci::{
    Event,
    EventAttributeIndexExt as _,
};
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    assets::StateReadExt as _,
    transaction::StateReadExt as _,
};

pub(crate) mod component;
mod state_ext;
pub(crate) mod storage;

pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

pub(crate) struct GenericFeeComponents {
    pub(crate) base_fee: u128,
    pub(crate) computed_cost_multiplier: u128,
}

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
const DEPOSIT_BASE_FEE: u128 = 16;

#[async_trait::async_trait]
pub(crate) trait FeeHandler {
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()>;

    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>>;

    fn computed_cost_base_component(&self) -> u128;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    asset: asset::Denom,
    amount: u128,
    source_transaction_id: TransactionId,
    source_action_index: u64,
}

impl Fee {
    pub(crate) fn asset(&self) -> &asset::Denom {
        &self.asset
    }

    pub(crate) fn amount(&self) -> u128 {
        self.amount
    }
}

#[async_trait::async_trait]
impl FeeHandler for Transfer {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let TransferFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_transfer_fees()
            .await
            .wrap_err("failed to get transfer fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeLock {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let BridgeLockFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_bridge_lock_fees()
            .await
            .wrap_err("failed to get bridge lock fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        calculate_base_deposit_fee(&Deposit {
            bridge_address: self.to,
            rollup_id: [0u8; 32].into(),
            amount: self.amount,
            asset: self.asset.clone(),
            destination_chain_address: self.destination_chain_address.clone(),
            source_transaction_id: TransactionId::new([0; TRANSACTION_ID_LEN]),
            source_action_index: 0,
        })
        .expect("deposit fee calculation should not fail")
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeSudoChange {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let BridgeSudoChangeFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_bridge_sudo_change_fees()
            .await
            .wrap_err("failed to get bridge sudo fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeUnlock {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let BridgeUnlockFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_bridge_unlock_fees()
            .await
            .wrap_err("failed to get bridge unlock fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for InitBridgeAccount {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let InitBridgeAccountFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_init_bridge_account_fees()
            .await
            .wrap_err("failed to get init bridge account fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for action::Ics20Withdrawal {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let Ics20WithdrawalFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_ics20_withdrawal_fees()
            .await
            .wrap_err("failed to get ics20 withdrawal fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for Sequence {
    #[instrument(skip_all, err)]
    async fn handle_fees_if_present<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fee_components = Self::fee_components(&state).await?;
        if fee_components.is_none() {
            return Ok(());
        }
        let fee_components =
            fee_components.expect("fee components should be present after checking for them");
        check_and_pay_fees(self, fee_components, state, &self.fee_asset).await
    }

    #[instrument(skip_all, err)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        let SequenceFeeComponents {
            base_fee,
            computed_cost_multiplier,
        } = state
            .get_sequence_fees()
            .await
            .wrap_err("failed to get sequence fees")?;
        Ok(Some(GenericFeeComponents {
            base_fee,
            computed_cost_multiplier,
        }))
    }

    #[instrument(skip_all)]
    fn computed_cost_base_component(&self) -> u128 {
        self.data
            .len()
            .try_into()
            .expect("a usize should always convert to a u128")
    }
}

#[async_trait::async_trait]
impl FeeHandler for ValidatorUpdate {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for SudoAddressChange {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeChange {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcSudoChange {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelayerChange {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeAssetChange {
    async fn handle_fees_if_present<S: StateWrite>(&self, _state: S) -> eyre::Result<()> {
        Ok(())
    }

    async fn fee_components<S: StateRead>(_state: S) -> eyre::Result<Option<GenericFeeComponents>> {
        Ok(None)
    }

    fn computed_cost_base_component(&self) -> u128 {
        0
    }
}

#[instrument(skip_all, err(level = Level::WARN))]
async fn check_and_pay_fees<S: StateWrite>(
    act: &impl FeeHandler,
    fee_components: GenericFeeComponents,
    mut state: S,
    fee_asset: &asset::Denom,
) -> astria_eyre::eyre::Result<()> {
    let total_fees = fee_components
        .base_fee
        .checked_add(
            act.computed_cost_base_component()
                .checked_mul(fee_components.computed_cost_multiplier)
                .ok_or_eyre("fee calculation overflow")?,
        )
        .ok_or_eyre("fee calculation overflow")?;
    let transaction_context = state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action");
    let from = transaction_context.address_bytes();
    let transaction_id = transaction_context.transaction_id;
    let source_action_index = transaction_context.source_action_index;

    ensure!(
        state
            .get_account_balance(&from, fee_asset)
            .await
            .wrap_err("failed to get account balance")?
            >= total_fees,
        "insufficient funds for transfer and fee payment",
    );
    ensure!(
        state
            .is_allowed_fee_asset(fee_asset)
            .await
            .wrap_err("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );
    state
        .add_fee_to_block_fees(fee_asset, total_fees, transaction_id, source_action_index)
        .wrap_err("failed to add to block fees")?;
    state
        .decrease_balance(&from, fee_asset, total_fees)
        .await
        .wrap_err("failed to decrease balance for fee payment")?;
    Ok(())
}

/// Returns a modified byte length of the deposit event. Length is calculated with reasonable values
/// for all fields except `asset` and `destination_chain_address`, ergo it may not be representative
/// of on-wire length.
pub(crate) fn calculate_base_deposit_fee(deposit: &Deposit) -> Option<u128> {
    deposit
        .asset
        .display_len()
        .checked_add(deposit.destination_chain_address.len())
        .and_then(|var_len| {
            DEPOSIT_BASE_FEE.checked_add(u128::try_from(var_len).expect(
                "converting a usize to a u128 should work on any currently existing machine",
            ))
        })
}

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
pub(crate) fn construct_tx_fee_event(fee: &Fee) -> Event {
    Event::new(
        "tx.fees",
        [
            ("asset", fee.asset.to_string()).index(),
            ("feeAmount", fee.amount.to_string()).index(),
            ("sourceTransactionId", fee.source_transaction_id.to_string()).index(),
            ("sourceActionIndex", fee.source_action_index.to_string()).index(),
        ],
    )
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
pub(crate) async fn calculate_sequence_action_fee_from_state<S: crate::fees::StateReadExt>(
    data: &[u8],
    state: &S,
) -> eyre::Result<u128> {
    let Some(GenericFeeComponents {
        base_fee,
        computed_cost_multiplier,
    }) = Sequence::fee_components(state).await?
    else {
        bail!("no fee components found for sequence action");
    };
    base_fee
        .checked_add(
            computed_cost_multiplier
                .checked_mul(
                    data.len()
                        .try_into()
                        .expect("a usize should always convert to a u128"),
                )
                .ok_or_eyre("fee multiplication component overflowed")?,
        )
        .ok_or_eyre("fee addition component overflowed")
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset::{
                self,
            },
            Address,
            RollupId,
            TransactionId,
            ADDRESS_LEN,
            ROLLUP_ID_LEN,
            TRANSACTION_ID_LEN,
        },
        protocol::transaction::v1alpha1::action::{
            BridgeLock,
            BridgeLockFeeComponents,
            TransferFeeComponents,
        },
        sequencerblock::v1alpha1::block::Deposit,
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::StateWriteExt as _,
        address::StateWriteExt as _,
        app::ActionHandler as _,
        assets::StateWriteExt as _,
        bridge::StateWriteExt as _,
        fees::{
            calculate_base_deposit_fee,
            StateWriteExt as _,
            DEPOSIT_BASE_FEE,
        },
        test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    fn test_asset() -> asset::Denom {
        "test".parse().unwrap()
    }

    #[tokio::test]
    async fn bridge_lock_fee_calculation_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;

        let from_address = astria_address(&[2; 20]);
        let transaction_id = TransactionId::new([0; 32]);
        state.put_transaction_context(TransactionContext {
            address_bytes: from_address.bytes(),
            transaction_id,
            source_action_index: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let transfer_fees = TransferFeeComponents {
            base_fee: transfer_fee,
            computed_cost_multiplier: 0,
        };
        state.put_transfer_fees(transfer_fees).unwrap();

        let bridge_lock_fees = BridgeLockFeeComponents {
            base_fee: transfer_fee,
            computed_cost_multiplier: 2,
        };
        state.put_bridge_lock_fees(bridge_lock_fees).unwrap();

        let bridge_address = astria_address(&[1; 20]);
        let asset = test_asset();
        let bridge_lock = BridgeLock {
            to: bridge_address,
            asset: asset.clone(),
            amount: 100,
            fee_asset: asset.clone(),
            destination_chain_address: "someaddress".to_string(),
        };

        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");
        state
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, asset.clone())
            .unwrap();
        state.put_allowed_fee_asset(&asset).unwrap();

        // not enough balance; should fail
        state
            .put_account_balance(&from_address, &asset, transfer_fee)
            .unwrap();
        assert_eyre_error(
            &bridge_lock.check_and_execute(&mut state).await.unwrap_err(),
            "insufficient funds for transfer",
        );

        // enough balance; should pass
        let expected_deposit_fee = transfer_fee
            + calculate_base_deposit_fee(&Deposit {
                bridge_address,
                rollup_id,
                amount: 100,
                asset: asset.clone(),
                destination_chain_address: "someaddress".to_string(),
                source_transaction_id: transaction_id,
                source_action_index: 0,
            })
            .unwrap()
                * 2;
        state
            .put_account_balance(&from_address, &asset, 100 + expected_deposit_fee)
            .unwrap();
        bridge_lock.check_and_execute(&mut state).await.unwrap();
    }

    #[test]
    fn calculated_base_deposit_fee_matches_expected_value() {
        assert_correct_base_deposit_fee(&Deposit {
            amount: u128::MAX,
            source_action_index: u64::MAX,
            ..reference_deposit()
        });
        assert_correct_base_deposit_fee(&Deposit {
            asset: "test_asset".parse().unwrap(),
            ..reference_deposit()
        });
        assert_correct_base_deposit_fee(&Deposit {
            destination_chain_address: "someaddresslonger".to_string(),
            ..reference_deposit()
        });

        // Ensure calculated length is as expected with absurd string
        // lengths (have tested up to 99999999, but this makes testing very slow)
        let absurd_string: String = ['a'; u16::MAX as usize].iter().collect();
        assert_correct_base_deposit_fee(&Deposit {
            asset: absurd_string.parse().unwrap(),
            ..reference_deposit()
        });
        assert_correct_base_deposit_fee(&Deposit {
            destination_chain_address: absurd_string,
            ..reference_deposit()
        });
    }

    #[track_caller]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "adding length of strings will never overflow u128 on currently existing machines"
    )]
    fn assert_correct_base_deposit_fee(deposit: &Deposit) {
        let calculated_len = calculate_base_deposit_fee(deposit).unwrap();
        let expected_len = DEPOSIT_BASE_FEE
            + deposit.asset.to_string().len() as u128
            + deposit.destination_chain_address.len() as u128;
        assert_eq!(calculated_len, expected_len);
    }

    /// Used to determine the base deposit byte length for `get_deposit_byte_len()`. This is based
    /// on "reasonable" values for all fields except `asset` and `destination_chain_address`. These
    /// are empty strings, whose length will be added to the base cost at the time of
    /// calculation.
    ///
    /// This test determines 165 bytes for an average deposit with empty `asset` and
    /// `destination_chain_address`, which is divided by 10 to get our base byte length of 16. This
    /// is to allow for more flexibility in overall fees (we have more flexibility multiplying by a
    /// lower number, and if we want fees to be higher we can just raise the multiplier).
    #[test]
    fn get_base_deposit_fee() {
        use prost::Message as _;
        let bridge_address = Address::builder()
            .prefix("astria-bridge")
            .slice(&[0u8; ADDRESS_LEN][..])
            .try_build()
            .unwrap();
        let raw_deposit = astria_core::generated::sequencerblock::v1alpha1::Deposit {
            bridge_address: Some(bridge_address.to_raw()),
            rollup_id: Some(RollupId::from_unhashed_bytes([0; ROLLUP_ID_LEN]).to_raw()),
            amount: Some(1000.into()),
            asset: String::new(),
            destination_chain_address: String::new(),
            source_transaction_id: Some(TransactionId::new([0; TRANSACTION_ID_LEN]).to_raw()),
            source_action_index: 0,
        };
        assert_eq!(DEPOSIT_BASE_FEE, raw_deposit.encoded_len() as u128 / 10);
    }

    fn reference_deposit() -> Deposit {
        Deposit {
            bridge_address: astria_address(&[1; 20]),
            rollup_id: RollupId::from_unhashed_bytes(b"test_rollup_id"),
            amount: 0,
            asset: "test".parse().unwrap(),
            destination_chain_address: "someaddress".to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        }
    }
}
