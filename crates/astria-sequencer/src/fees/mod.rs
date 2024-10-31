use astria_core::{
    primitive::v1::asset,
    protocol::transaction::{
        self,
        v1::{
            action::{
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
            Action,
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

use crate::accounts::StateWriteExt as _;

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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()>;

    fn variable_component(&self) -> u128;
}

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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_transfer_fees()
            .await
            .wrap_err("error fetching transfer fees")?
            .ok_or_eyre("transfer fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for Action {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        match self {
            Action::RollupDataSubmission(act) => act.check_and_pay_fees(state, context).await,
            Action::Transfer(act) => act.check_and_pay_fees(state, context).await,
            Action::ValidatorUpdate(act) => act.check_and_pay_fees(state, context).await,
            Action::SudoAddressChange(act) => act.check_and_pay_fees(state, context).await,
            Action::Ibc(act) => act.check_and_pay_fees(state, context).await,
            Action::IbcSudoChange(act) => act.check_and_pay_fees(state, context).await,
            Action::Ics20Withdrawal(act) => act.check_and_pay_fees(state, context).await,
            Action::IbcRelayerChange(act) => act.check_and_pay_fees(state, context).await,
            Action::FeeAssetChange(act) => act.check_and_pay_fees(state, context).await,
            Action::InitBridgeAccount(act) => act.check_and_pay_fees(state, context).await,
            Action::BridgeLock(act) => act.check_and_pay_fees(state, context).await,
            Action::BridgeUnlock(act) => act.check_and_pay_fees(state, context).await,
            Action::BridgeSudoChange(act) => act.check_and_pay_fees(state, context).await,
            Action::FeeChange(act) => act.check_and_pay_fees(state, context).await,
        }
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        match self {
            Action::RollupDataSubmission(act) => act.variable_component(),
            Action::Transfer(act) => act.variable_component(),
            Action::ValidatorUpdate(act) => act.variable_component(),
            Action::SudoAddressChange(act) => act.variable_component(),
            Action::Ibc(act) => act.variable_component(),
            Action::IbcSudoChange(act) => act.variable_component(),
            Action::Ics20Withdrawal(act) => act.variable_component(),
            Action::IbcRelayerChange(act) => act.variable_component(),
            Action::FeeAssetChange(act) => act.variable_component(),
            Action::InitBridgeAccount(act) => act.variable_component(),
            Action::BridgeLock(act) => act.variable_component(),
            Action::BridgeUnlock(act) => act.variable_component(),
            Action::BridgeSudoChange(act) => act.variable_component(),
            Action::FeeChange(act) => act.variable_component(),
        }
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeLock {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_bridge_lock_fees()
            .await
            .wrap_err("error fetching bridge lock fees")?
            .ok_or_eyre("bridge lock fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        base_deposit_fee(&self.asset, &self.destination_chain_address)
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeSudoChange {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_bridge_sudo_change_fees()
            .await
            .wrap_err("error fetching bridge sudo change fees")?
            .ok_or_eyre("bridge sudo change fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for BridgeUnlock {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_bridge_unlock_fees()
            .await
            .wrap_err("error fetching bridge unlock fees")?
            .ok_or_eyre("bridge unlock fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for InitBridgeAccount {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_init_bridge_account_fees()
            .await
            .wrap_err("error fetching init bridge account fees")?
            .ok_or_eyre("init bridge account fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for transaction::v1::action::Ics20Withdrawal {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_ics20_withdrawal_fees()
            .await
            .wrap_err("error fetching ics20 withdrawal fees")?
            .ok_or_eyre("ics20 withdrawal fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
    }

    #[instrument(skip_all)]
    fn variable_component(&self) -> u128 {
        0
    }
}

#[async_trait::async_trait]
impl FeeHandler for RollupDataSubmission {
    #[instrument(skip_all, err)]
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> eyre::Result<()> {
        let fees = state
            .get_rollup_data_submission_fees()
            .await
            .wrap_err("error fetching rollup data submission fees")?
            .ok_or_eyre("rollup data submission fees not found, so this action is disabled")?;
        check_and_pay_fees(
            self,
            fees.base,
            fees.multiplier,
            state,
            &self.fee_asset,
            context,
        )
        .await
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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
    async fn check_and_pay_fees<S: StateWrite>(
        &self,
        state: S,
        _context: crate::transaction::Context,
    ) -> eyre::Result<()> {
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

#[instrument(skip_all, err(level = Level::WARN))]
async fn check_and_pay_fees<S: StateWrite, T: FeeHandler + Protobuf>(
    act: &T,
    base: u128,
    multiplier: u128,
    mut state: S,
    fee_asset: &asset::Denom,
    context: crate::transaction::Context,
) -> eyre::Result<()> {
    let total_fees = base.saturating_add(act.variable_component().saturating_mul(multiplier));

    ensure!(
        state
            .is_allowed_fee_asset(fee_asset)
            .await
            .wrap_err("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );
    state
        .add_fee_to_block_fees::<_, T>(fee_asset, total_fees, context.source_action_index)
        .wrap_err("failed to add to block fees")?;
    state
        .decrease_balance(&context.address_bytes, fee_asset, total_fees)
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
