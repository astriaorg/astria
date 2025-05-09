use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset::{
        Denom,
        IbcPrefixed,
    },
    protocol::fees::v1::FeeComponents,
};
use astria_eyre::eyre::Report;
use cnidarium::StateRead;
use tracing::{
    instrument,
    Level,
};

use super::CheckedActionFeeError;
use crate::{
    checked_actions::ActionRef,
    fees::{
        FeeHandler,
        StateReadExt as _,
    },
    storage::StoredValue,
};

/// Returns the total of all fees associated with executing the given actions.
#[instrument(skip_all, err(level = Level::DEBUG))]
pub(crate) async fn total_fees<'a, I: Iterator<Item = ActionRef<'a>>, S: StateRead>(
    actions: I,
    state: &S,
) -> Result<HashMap<IbcPrefixed, u128>, CheckedActionFeeError> {
    let mut fees_by_asset = HashMap::new();
    for action in actions {
        let maybe_fee = match action {
            ActionRef::RollupDataSubmission(action) => fee(action, state).await,
            ActionRef::Transfer(action) => fee(action, state).await,
            ActionRef::ValidatorUpdate(action) => fee(action, state).await,
            ActionRef::SudoAddressChange(action) => fee(action, state).await,
            ActionRef::Ibc(action) => fee(action, state).await,
            ActionRef::IbcSudoChange(action) => fee(action, state).await,
            ActionRef::Ics20Withdrawal(action) => fee(action, state).await,
            ActionRef::IbcRelayerChange(action) => fee(action, state).await,
            ActionRef::FeeAssetChange(action) => fee(action, state).await,
            ActionRef::InitBridgeAccount(action) => fee(action, state).await,
            ActionRef::BridgeLock(action) => fee(action, state).await,
            ActionRef::BridgeUnlock(action) => fee(action, state).await,
            ActionRef::BridgeSudoChange(action) => fee(action, state).await,
            ActionRef::BridgeTransfer(action) => fee(action, state).await,
            ActionRef::FeeChange(action) => fee(action, state).await,
            ActionRef::RecoverIbcClient(action) => fee(action, state).await,
            ActionRef::CurrencyPairsChange(action) => fee(action, state).await,
            ActionRef::MarketsChange(action) => fee(action, state).await,
        }?;
        let Some((fee_asset, fee_amount)) = maybe_fee else {
            // If there's no fee asset, we don't charge fees.
            continue;
        };
        fees_by_asset
            .entry(fee_asset.to_ibc_prefixed())
            .and_modify(|amt: &mut u128| *amt = amt.saturating_add(fee_amount))
            .or_insert(fee_amount);
    }
    Ok(fees_by_asset)
}

#[instrument(skip_all, err(level = Level::DEBUG))]
pub(super) async fn fee<'a, F, S>(
    action: &'a F,
    state: &S,
) -> Result<Option<(&'a Denom, u128)>, CheckedActionFeeError>
where
    F: FeeHandler,
    S: StateRead,
    FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report>,
{
    let fees = state
        .get_fees::<F>()
        .await
        .map_err(|source| {
            CheckedActionFeeError::internal(
                &format!("error fetching {} fees", action.name()),
                source,
            )
        })?
        .ok_or_else(|| CheckedActionFeeError::ActionDisabled {
            action_name: action.name(),
        })?;
    let Some(fee_asset) = action.fee_asset() else {
        // If the action has no associated fee asset, there are no fees to pay.
        return Ok(None);
    };

    let is_allowed_fee_asset = state
        .is_allowed_fee_asset(fee_asset)
        .await
        .map_err(|source| {
            CheckedActionFeeError::internal("failed to check allowed fee assets in state", source)
        })?;
    if !is_allowed_fee_asset {
        return Err(CheckedActionFeeError::FeeAssetIsNotAllowed {
            fee_asset: fee_asset.clone(),
            action_name: action.name(),
        });
    }

    let variable_fee = action
        .variable_component()
        .saturating_mul(fees.multiplier());
    let total_fee = fees.base().saturating_add(variable_fee);
    Ok(Some((fee_asset, total_fee)))
}
