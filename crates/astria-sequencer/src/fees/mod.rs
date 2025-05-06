use astria_core::{
    primitive::v1::asset,
    protocol::{
        fees::v1::FeeComponents,
        transaction::v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeTransfer,
            BridgeUnlock,
            CurrencyPairsChange,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            MarketsChange,
            RecoverIbcClient,
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
    eyre,
    Report,
    WrapErr as _,
};
use cnidarium::StateWrite;
use penumbra_ibc::IbcRelay;
use prost::Name;
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::StateWriteExt as _,
    transaction::StateReadExt as _,
};

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

use crate::storage::StoredValue;

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
const DEPOSIT_BASE_FEE: u128 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    action_name: String,
    asset: asset::Denom,
    amount: u128,
    position_in_transaction: u64,
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
pub(crate) trait FeeHandler: Send {
    /// The Pascal-case type name, e.g. `RollupDataSubmission`.
    // NOTE: We only require this function due to `IbcRelay` not implementing `Protobuf`.
    fn name() -> &'static str;

    /// The full name including the protobuf package, e.g.
    /// `astria.protocol.transaction.v1.RollupDataSubmission`.
    // NOTE: We only require this function due to `IbcRelay` not implementing `Protobuf`.
    fn full_name() -> String;

    /// The snake-case type name, e.g. `rollup_data_submission`.
    fn snake_case_name() -> &'static str;

    /// The variable value derived from `self` which is multiplied by the `multiplier` of the
    /// `FeeComponents` for this action to produce the variable portion of the total fees for this
    /// action.
    ///
    /// Many actions have fixed fees, meaning this method returns `0`.
    fn variable_component(&self) -> u128;

    /// The asset to be used to pay the fees.
    ///
    /// If this method returns `None`, the action is free.
    fn fee_asset(&self) -> Option<&asset::Denom>;

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_pay_fees<'a, S>(&self, mut state: S) -> eyre::Result<()>
    where
        S: StateWrite,
        FeeComponents<Self>: TryFrom<StoredValue<'a>, Error = Report>,
    {
        let fees = state
            .get_fees::<Self>()
            .await
            .wrap_err_with(|| format!("error fetching {} fees", Self::name()))?
            .ok_or_else(|| {
                eyre!(
                    "{} fees not found, so this action is disabled",
                    Self::name()
                )
            })?;
        let Some(fee_asset) = self.fee_asset() else {
            // If the action has no associated fee asset, there are no fees to pay.
            return Ok(());
        };

        ensure!(
            state
                .is_allowed_fee_asset(fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        let variable_fee = self.variable_component().saturating_mul(fees.multiplier());
        let total_fees = fees.base().saturating_add(variable_fee);
        let transaction_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = transaction_context.address_bytes();
        let position_in_transaction = transaction_context.position_in_transaction;

        state.add_fee_to_block_fees::<_, Self>(fee_asset, total_fees, position_in_transaction);
        state
            .decrease_balance(&from, fee_asset, total_fees)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        Ok(())
    }
}

impl FeeHandler for Transfer {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "transfer"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeLock {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_lock"
    }

    fn variable_component(&self) -> u128 {
        base_deposit_fee(&self.asset, &self.destination_chain_address)
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeSudoChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_sudo_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeUnlock {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_unlock"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeTransfer {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_transfer"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for InitBridgeAccount {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "init_bridge_account"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for Ics20Withdrawal {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ics20_withdrawal"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for RollupDataSubmission {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "rollup_data_submission"
    }

    fn variable_component(&self) -> u128 {
        u128::try_from(self.data.len())
            .expect("converting a usize to a u128 should work on any currently existing machine")
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for ValidatorUpdate {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "validator_update"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for SudoAddressChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "sudo_address_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for FeeChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "fee_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for IbcSudoChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_sudo_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for IbcRelayerChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_relayer_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for FeeAssetChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "fee_asset_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for IbcRelay {
    fn name() -> &'static str {
        penumbra_proto::penumbra::core::component::ibc::v1::IbcRelay::NAME
    }

    fn full_name() -> String {
        penumbra_proto::penumbra::core::component::ibc::v1::IbcRelay::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_relay"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for RecoverIbcClient {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "recover_ibc_client"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for CurrencyPairsChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "currency_pairs_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
}

impl FeeHandler for MarketsChange {
    fn name() -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "markets_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&asset::Denom> {
        None
    }
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
