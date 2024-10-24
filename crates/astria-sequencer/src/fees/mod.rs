use access::FeeComponents;
use astria_core::{
    primitive::v1::{
        asset,
        TransactionId,
    },
    protocol::{
        fees::v1::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            BridgeUnlockFeeComponents,
            FeeAssetChangeFeeComponents,
            FeeChangeFeeComponents,
            IbcRelayFeeComponents,
            IbcRelayerChangeFeeComponents,
            IbcSudoChangeFeeComponents,
            Ics20WithdrawalFeeComponents,
            InitBridgeAccountFeeComponents,
            RollupDataSubmissionFeeComponents,
            SudoAddressChangeFeeComponents,
            TransferFeeComponents,
            ValidatorUpdateFeeComponents,
        },
        transaction::{
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
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    ensure,
    OptionExt as _,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use penumbra_ibc::IbcRelay;
use tendermint::abci::{
    Event,
    EventAttributeIndexExt as _,
};
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::StateWriteExt as _,
    transaction::StateReadExt as _,
};

pub(crate) mod access;
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

#[async_trait::async_trait]
pub(crate) trait FeeHandler {
    type FeeComponents: FeeComponents;

    async fn check_and_pay_fees<S: StateWrite>(&self, state: S) -> eyre::Result<()>;

    fn variable_component(&self) -> u128;

    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    action_name: String,
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
    type FeeComponents = TransferFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_transfer_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeLock {
    type FeeComponents = BridgeLockFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_bridge_lock_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeSudoChange {
    type FeeComponents = BridgeSudoChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_bridge_sudo_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeUnlock {
    type FeeComponents = BridgeUnlockFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_bridge_unlock_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for InitBridgeAccount {
    type FeeComponents = InitBridgeAccountFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_init_bridge_account_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for transaction::v1::action::Ics20Withdrawal {
    type FeeComponents = Ics20WithdrawalFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_ics20_withdrawal_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for RollupDataSubmission {
    type FeeComponents = RollupDataSubmissionFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_rollup_data_submission_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for ValidatorUpdate {
    type FeeComponents = ValidatorUpdateFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_validator_update_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for SudoAddressChange {
    type FeeComponents = SudoAddressChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_sudo_address_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeChange {
    type FeeComponents = FeeChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_fee_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcSudoChange {
    type FeeComponents = IbcSudoChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_ibc_sudo_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelayerChange {
    type FeeComponents = IbcRelayerChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_ibc_relayer_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for FeeAssetChange {
    type FeeComponents = FeeAssetChangeFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_fee_asset_change_fees().await
    }
}

#[async_trait::async_trait]
impl FeeHandler for IbcRelay {
    type FeeComponents = IbcRelayFeeComponents;

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

    #[instrument(skip_all)]
    async fn fee_components<S: StateRead>(state: S) -> eyre::Result<Option<Self::FeeComponents>> {
        state.get_ibc_relay_fees().await
    }
}

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
    let transaction_id = transaction_context.transaction_id;
    let source_action_index = transaction_context.source_action_index;

    ensure!(
        state
            .is_allowed_fee_asset(fee_asset)
            .await
            .wrap_err("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );
    state
        .add_fee_to_block_fees::<_, T>(fee_asset, total_fees, transaction_id, source_action_index)
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
fn base_deposit_fee(asset: &asset::Denom, destination_chain_address: &str) -> u128 {
    u128::try_from(
        asset
            .display_len()
            .saturating_add(destination_chain_address.len()),
    )
    .expect("converting a usize to a u128 should work on any currently existing machine")
    .saturating_add(DEPOSIT_BASE_FEE)
}

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
pub(crate) fn construct_tx_fee_event(fee: &Fee) -> Event {
    Event::new(
        "tx.fees",
        [
            ("actionName", fee.action_name.to_string()).index(),
            ("asset", fee.asset.to_string()).index(),
            ("feeAmount", fee.amount.to_string()).index(),
            ("sourceTransactionId", fee.source_transaction_id.to_string()).index(),
            ("sourceActionIndex", fee.source_action_index.to_string()).index(),
        ],
    )
}
