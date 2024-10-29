use astria_core::{
    primitive::v1::asset,
    protocol::transaction::{
        self,
        v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    ensure,
    OptionExt as _,
    WrapErr as _,
};
use cnidarium::StateWrite;
use penumbra_ibc::IbcRelay;
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::StateWriteExt as _,
    transaction::StateReadExt as _,
};

pub(crate) mod action;
pub(crate) mod component;
pub(crate) mod query;
mod state_ext;
pub(crate) mod storage;

#[cfg(test)]
mod tests;

pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
const DEPOSIT_BASE_FEE: u128 = 16;

// This module contains the fee handling logic for all actions. All actions must implement the
// [`FeeHandler`] trait. In addition to this, new actions should provide new, unique read/write
// methods in [`fees::StateReadExt`] and [`fees::StateWriteExt`] for the fees associated with the
// action, which take/return a domain type [`<action_name>FeeComponents`]. If there are no fees
// stored to the state for the given action, it is assumed to be disabled, and the transaction
// execution should fail. If the action is free, the fees should be explicitly set to zero in the
// state. See action implementations for examples of how to implement this trait for new actions.

/// This trait handles fees for all actions, even if the given aciton is free. It must
/// be implemented for all actions. All actions' fees are calculated via the formula:
/// `base + (variable * multiplier)`.
#[async_trait::async_trait]
pub(crate) trait FeeHandler {
    /// Gets the associated fees for the action and pays them. If the action is free, the
    /// implementation of this method should still attempt to read the zero fees from state. If
    /// no fees are found, the action is de facto disabled, and the implementation should error
    /// out.
    // NOTE: All fee-bearing implementations of this method MUST make use of the private helper
    // function `check_and_pay_fees`.
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()>;

    /// Returns the variable component of the fee calculation: `base + (variable * multiplier)`.
    fn variable_component(&self) -> u128;
}

/// Contains all the necessary information to mint fees in the receiver's account and to construct a
/// `tx.fees` ABCI event for sequencer fee reporting. `Fee`s are added to the block fees as they are
/// deducted from the payer's account in [`FeeHandler::check_and_pay_fees`]. The fees are then paid
/// to the payee and events created in [`crate::app::end_block`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    action_name: String,
    asset: asset::Denom,
    amount: u128,
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
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_transfer_fees()
            .await
            .wrap_err("error fetching transfer fees")?
            .ok_or_eyre("transfer fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeLock {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_bridge_lock_fees()
            .await
            .wrap_err("error fetching bridge lock fees")?
            .ok_or_eyre("bridge lock fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        base_deposit_fee(&self.asset, &self.destination_chain_address)
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeSudoChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_bridge_sudo_change_fees()
            .await
            .wrap_err("error fetching bridge sudo change fees")?
            .ok_or_eyre("bridge sudo change fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeUnlock {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_bridge_unlock_fees()
            .await
            .wrap_err("error fetching bridge unlock fees")?
            .ok_or_eyre("bridge unlock fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for InitBridgeAccount {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_init_bridge_account_fees()
            .await
            .wrap_err("error fetching init bridge account fees")?
            .ok_or_eyre("init bridge account fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for transaction::v1::action::Ics20Withdrawal {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_ics20_withdrawal_fees()
            .await
            .wrap_err("error fetching ics20 withdrawal fees")?
            .ok_or_eyre("ics20 withdrawal fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for RollupDataSubmission {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        let fees = state
            .get_rollup_data_submission_fees()
            .await
            .wrap_err("error fetching rollup data submission fees")?
            .ok_or_eyre("rollup data submission fees not found, so this action is disabled")?;
        check_and_pay_fees(self, fees.base, fees.multiplier, state, &self.fee_asset).await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        u128::try_from(self.data.len())
            .expect("converting a usize to a u128 should work on any currently existing machine")
    }
}

#[async_trait::async_trait]
impl FeeHandler for ValidatorUpdate {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_validator_update_fees()
            .await
            .wrap_err("error fetching validator update fees")?
            .ok_or_eyre("validator update fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for SudoAddressChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_sudo_address_change_fees()
            .await
            .wrap_err("error fetching sudo address change fees")?
            .ok_or_eyre("sudo address change fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_fee_change_fees()
            .await
            .wrap_err("error fetching fee change fees")?
            .ok_or_eyre("fee change fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcSudoChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_ibc_sudo_change_fees()
            .await
            .wrap_err("error fetching ibc sudo change fees")?
            .ok_or_eyre("ibc sudo change fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelayerChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_ibc_relayer_change_fees()
            .await
            .wrap_err("error fetching ibc relayer change fees")?
            .ok_or_eyre("ibc relayer change fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeAssetChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_fee_asset_change_fees()
            .await
            .wrap_err("error fetching fee asset change fees")?
            .ok_or_eyre("fee asset change fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelay {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()> {
        state
            .get_ibc_relay_fees()
            .await
            .wrap_err("error fetching ibc relay fees")?
            .ok_or_eyre("ibc relay fees not found, so this action is disabled")?;
        Ok(())
    }

    fn variable_component(&self) -> u128 {
        0
    }
}

/// Takes an action, its fees, the state, and the fee asset, and pays the fees. Fees are calculated
/// via the formula `base + (variable * multiplier)`. This only deducts the fees from the sender's
/// balance, but does not mint them in the receiver's account. This action takes place in
/// [`crate::app::end_block`].
///
/// # Errors
///
/// - If the function fails to get allowed fee assets from the state.
/// - If the fee asset is not allowed in the state.
/// - If deduction of the fees from the sender's balance fails.
/// - If adding the fees event to the block fees fails.
#[instrument(skip_all, err(level = Level::WARN))]
async fn check_and_pay_fees<S: StateWrite, T: FeeHandler + Protobuf>(
    act: &T,
    base: u128,
    multiplier: u128,
    mut state: S,
    fee_asset: &asset::Denom,
) -> eyre::Result<()> {
    let total_fees = base.saturating_add(act.variable_component().saturating_mul(multiplier));
    let transaction_context = state
        .get_transaction_context()
        .expect("transaction source must be present in state when executing an action");
    let from = transaction_context.address_bytes();
    let source_action_index = transaction_context.source_action_index;

    ensure!(
        state
            .is_allowed_fee_asset(fee_asset)
            .await
            .wrap_err("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );
    state
        .add_fee_to_block_fees::<_, T>(fee_asset, total_fees, source_action_index)
        .wrap_err("failed to add to block fees")?;
    state
        .decrease_balance(&from, fee_asset, total_fees)
        .await
        .wrap_err("failed to decrease balance for fee payment")?;
    Ok(())
}

/// Returns a modified byte length of the deposit event. Length is calculated by adding
/// the lengths of the asset and the destination chain address to the base deposit fee.
fn base_deposit_fee(asset: &asset::Denom, destination_chain_address: &str) -> u128 {
    u128::try_from(
        asset
            .display_len()
            .saturating_add(destination_chain_address.len()),
    )
    .expect("converting a usize to a u128 should work on any currently existing machine")
    .saturating_add(DEPOSIT_BASE_FEE)
}
