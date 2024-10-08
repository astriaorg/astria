use astria_core::{
    primitive::v1::{
        asset,
        TransactionId,
    },
    protocol::transaction::v1alpha1::action::{
        self,
        BridgeLock,
        BridgeSudoChange,
        BridgeUnlock,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        InitBridgeAccount,
        Sequence,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;
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
    bridge::StateReadExt as _,
    ibc::StateReadExt as _,
    sequence,
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

mod state_ext;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
const DEPOSIT_BASE_FEE: u128 = 16;

#[async_trait::async_trait]
pub(crate) trait FeeHandler {
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        mut state: S,
    ) -> astria_eyre::eyre::Result<()>;
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
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let fee = state
            .get_transfer_base_fee()
            .await
            .wrap_err("failed to get transfer base fee")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        state.add_fee_to_block_fees(
            &self.fee_asset,
            fee,
            tx_context.transaction_id,
            tx_context.source_action_index,
        )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeLock {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let rollup_id = state
            .get_bridge_account_rollup_id(&self.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;
        let transfer_fee = state
            .get_transfer_base_fee()
            .await
            .context("failed to get transfer base fee")?;
        let from = tx_context.address_bytes();
        let source_transaction_id = tx_context.transaction_id;
        let source_action_index = tx_context.source_action_index;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        let deposit = Deposit {
            bridge_address: self.to,
            rollup_id,
            amount: self.amount,
            asset: self.asset.clone(),
            destination_chain_address: self.destination_chain_address.clone(),
            source_transaction_id,
            source_action_index,
        };
        let deposit_abci_event = create_deposit_event(&deposit);

        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .wrap_err("failed to get byte cost multiplier")?;

        let fee = byte_cost_multiplier
            .saturating_mul(calculate_base_deposit_fee(&deposit).unwrap_or(u128::MAX))
            .saturating_add(transfer_fee);

        state
            .add_fee_to_block_fees(
                &self.fee_asset,
                fee,
                tx_context.transaction_id,
                source_action_index,
            )
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to deduct fee from account balance")?;

        state.record(deposit_abci_event);
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeSudoChange {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let fee = state
            .get_bridge_sudo_change_base_fee()
            .await
            .wrap_err("failed to get bridge sudo change fee")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .add_fee_to_block_fees(
                &self.fee_asset,
                fee,
                tx_context.transaction_id,
                tx_context.source_action_index,
            )
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for bridge sudo change fee")?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeUnlock {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let transfer_fee = state
            .get_transfer_base_fee()
            .await
            .wrap_err("failed to get transfer base fee for bridge unlock action")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .decrease_balance(&from, &self.fee_asset, transfer_fee)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        state.add_fee_to_block_fees(
            &self.fee_asset,
            transfer_fee,
            tx_context.transaction_id,
            tx_context.source_action_index,
        )?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for InitBridgeAccount {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let fee = state
            .get_init_bridge_account_base_fee()
            .await
            .wrap_err("failed to get init bridge account base fee")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        state.add_fee_to_block_fees(
            &self.fee_asset,
            fee,
            tx_context.transaction_id,
            tx_context.source_action_index,
        )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for action::Ics20Withdrawal {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let fee = state
            .get_ics20_withdrawal_base_fee()
            .await
            .wrap_err("failed to get ics20 withdrawal base fee")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        state.add_fee_to_block_fees(
            &self.fee_asset,
            fee,
            tx_context.transaction_id,
            tx_context.source_action_index,
        )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for Sequence {
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn check_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed accessing state to check if fee is allowed")?,
            "invalid fee asset",
        );

        let fee = calculate_sequence_action_fee_from_state(&self.data, &state)
            .await
            .wrap_err("calculated fee overflows u128")?;

        state
            .add_fee_to_block_fees(
                &self.fee_asset,
                fee,
                tx_context.transaction_id,
                tx_context.source_action_index,
            )
            .wrap_err("failed to add to block fees")?;
        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed updating `from` account balance")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for ValidatorUpdate {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for SudoAddressChange {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeChange {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcSudoChange {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelayerChange {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeAssetChange {
    async fn check_and_pay_fees<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
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

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
pub(crate) async fn calculate_sequence_action_fee_from_state<S: sequence::StateReadExt>(
    data: &[u8],
    state: &S,
) -> Result<u128> {
    let base_fee = state
        .get_sequence_action_base_fee()
        .await
        .wrap_err("failed to get base fee")?;
    let fee_per_byte = state
        .get_sequence_action_byte_cost_multiplier()
        .await
        .wrap_err("failed to get fee per byte")?;
    calculate_sequence_action_fee(data, fee_per_byte, base_fee)
        .ok_or_eyre("calculated fee overflows u128")
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
/// Returns `None` if the fee overflows `u128`.
fn calculate_sequence_action_fee(data: &[u8], fee_per_byte: u128, base_fee: u128) -> Option<u128> {
    base_fee.checked_add(
        fee_per_byte.checked_mul(
            data.len()
                .try_into()
                .expect("a usize should always convert to a u128"),
        )?,
    )
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
        protocol::transaction::v1alpha1::action::BridgeLock,
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
            calculate_sequence_action_fee,
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

        state.put_transfer_base_fee(transfer_fee).unwrap();
        state.put_bridge_lock_byte_cost_multiplier(2).unwrap();

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
            "insufficient funds for transfer and fee payment",
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

    #[test]
    fn calculate_sequence_action_fee_works_as_expected() {
        assert_eq!(calculate_sequence_action_fee(&[], 1, 0), Some(0));
        assert_eq!(calculate_sequence_action_fee(&[0], 1, 0), Some(1));
        assert_eq!(calculate_sequence_action_fee(&[0u8; 10], 1, 0), Some(10));
        assert_eq!(calculate_sequence_action_fee(&[0u8; 10], 1, 100), Some(110));
    }
}
