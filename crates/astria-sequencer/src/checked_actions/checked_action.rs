use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
        ADDRESS_LEN,
    },
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
    upgrades::v1::blackburn::{
        AllowIbcRelayToFail,
        Blackburn,
    },
};
use astria_eyre::eyre;
use cnidarium::{
    StateRead,
    StateWrite,
};
use penumbra_ibc::IbcRelay;
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    CheckedActionExecutionError,
    CheckedActionFeeError,
    CheckedActionInitialCheckError,
    CheckedActionMutableCheckError,
    CheckedBridgeLock,
    CheckedBridgeSudoChange,
    CheckedBridgeTransfer,
    CheckedBridgeUnlock,
    CheckedCurrencyPairsChange,
    CheckedFeeAssetChange,
    CheckedFeeChange,
    CheckedIbcRelay,
    CheckedIbcRelayerChange,
    CheckedIbcSudoChange,
    CheckedIcs20Withdrawal,
    CheckedInitBridgeAccount,
    CheckedMarketsChange,
    CheckedRecoverIbcClient,
    CheckedRollupDataSubmission,
    CheckedSudoAddressChange,
    CheckedTransfer,
    CheckedValidatorUpdate,
};
use crate::{
    accounts::{
        InsufficientFunds,
        StateWriteExt as _,
    },
    fees::{
        FeeHandler,
        StateWriteExt as _,
    },
    ibc::StateWriteExt as _,
    storage::StoredValue,
    upgrades::StateReadExt as _,
};

/// An enum of all the various checked action types.
#[derive(Debug)]
pub(crate) enum CheckedAction {
    RollupDataSubmission(CheckedRollupDataSubmission),
    Transfer(CheckedTransfer),
    ValidatorUpdate(CheckedValidatorUpdate),
    SudoAddressChange(CheckedSudoAddressChange),
    IbcRelay(CheckedIbcRelay),
    IbcSudoChange(CheckedIbcSudoChange),
    Ics20Withdrawal(Box<CheckedIcs20Withdrawal>),
    IbcRelayerChange(CheckedIbcRelayerChange),
    FeeAssetChange(CheckedFeeAssetChange),
    InitBridgeAccount(CheckedInitBridgeAccount),
    BridgeLock(CheckedBridgeLock),
    BridgeUnlock(CheckedBridgeUnlock),
    BridgeSudoChange(CheckedBridgeSudoChange),
    BridgeTransfer(Box<CheckedBridgeTransfer>),
    FeeChange(CheckedFeeChange),
    RecoverIbcClient(CheckedRecoverIbcClient),
    CurrencyPairsChange(CheckedCurrencyPairsChange),
    MarketsChange(CheckedMarketsChange),
}

impl CheckedAction {
    pub(crate) fn new_rollup_data_submission(
        action: RollupDataSubmission,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedRollupDataSubmission::new(action)
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::RollupDataSubmission(checked_action))
    }

    pub(crate) async fn new_transfer<S: StateRead>(
        action: Transfer,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedTransfer::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::Transfer(checked_action))
    }

    pub(crate) async fn new_validator_update<S: StateRead>(
        action: ValidatorUpdate,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedValidatorUpdate::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::ValidatorUpdate(checked_action))
    }

    pub(crate) async fn new_sudo_address_change<S: StateRead>(
        action: SudoAddressChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedSudoAddressChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::SudoAddressChange(checked_action))
    }

    pub(crate) async fn new_ibc_relay<S: StateRead>(
        action: IbcRelay,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedIbcRelay::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::IbcRelay(checked_action))
    }

    pub(crate) async fn new_ibc_sudo_change<S: StateRead>(
        action: IbcSudoChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedIbcSudoChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::IbcSudoChange(checked_action))
    }

    pub(crate) async fn new_ics20_withdrawal<S: StateRead>(
        action: Ics20Withdrawal,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedIcs20Withdrawal::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::Ics20Withdrawal(Box::new(checked_action)))
    }

    pub(crate) async fn new_ibc_relayer_change<S: StateRead>(
        action: IbcRelayerChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedIbcRelayerChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::IbcRelayerChange(checked_action))
    }

    pub(crate) async fn new_fee_asset_change<S: StateRead>(
        action: FeeAssetChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedFeeAssetChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::FeeAssetChange(checked_action))
    }

    pub(crate) async fn new_init_bridge_account<S: StateRead>(
        action: InitBridgeAccount,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedInitBridgeAccount::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::InitBridgeAccount(checked_action))
    }

    pub(crate) async fn new_bridge_lock<S: StateRead>(
        action: BridgeLock,
        tx_signer: [u8; ADDRESS_LEN],
        tx_id: TransactionId,
        position_in_tx: u64,
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action =
            CheckedBridgeLock::new(action, tx_signer, tx_id, position_in_tx, state)
                .await
                .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::BridgeLock(checked_action))
    }

    pub(crate) async fn new_bridge_unlock<S: StateRead>(
        action: BridgeUnlock,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedBridgeUnlock::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::BridgeUnlock(checked_action))
    }

    pub(crate) async fn new_bridge_sudo_change<S: StateRead>(
        action: BridgeSudoChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedBridgeSudoChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::BridgeSudoChange(checked_action))
    }

    pub(crate) async fn new_bridge_transfer<S: StateRead>(
        action: BridgeTransfer,
        tx_signer: [u8; ADDRESS_LEN],
        tx_id: TransactionId,
        position_in_tx: u64,
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action =
            CheckedBridgeTransfer::new(action, tx_signer, tx_id, position_in_tx, state)
                .await
                .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::BridgeTransfer(Box::new(checked_action)))
    }

    pub(crate) async fn new_fee_change<S: StateRead>(
        action: FeeChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedFeeChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::FeeChange(checked_action))
    }

    pub(crate) async fn new_recover_ibc_client<S: StateRead>(
        action: RecoverIbcClient,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedRecoverIbcClient::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::RecoverIbcClient(checked_action))
    }

    pub(crate) async fn new_currency_pairs_change<S: StateRead>(
        action: CurrencyPairsChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedCurrencyPairsChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::CurrencyPairsChange(checked_action))
    }

    pub(crate) async fn new_markets_change<S: StateRead>(
        action: MarketsChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        let action_name = action.name();
        let checked_action = CheckedMarketsChange::new(action, tx_signer, state)
            .await
            .map_err(|source| CheckedActionInitialCheckError::new(action_name, source))?;
        Ok(Self::MarketsChange(checked_action))
    }

    pub(crate) async fn run_mutable_checks<S: StateRead>(
        &self,
        state: S,
    ) -> Result<(), CheckedActionMutableCheckError> {
        match self {
            Self::RollupDataSubmission(_checked_action) => Ok(()),
            Self::Transfer(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::ValidatorUpdate(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::SudoAddressChange(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::IbcRelay(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::IbcSudoChange(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::Ics20Withdrawal(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::IbcRelayerChange(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::FeeAssetChange(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::InitBridgeAccount(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::BridgeLock(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::BridgeUnlock(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::BridgeSudoChange(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::BridgeTransfer(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::FeeChange(checked_action) => checked_action.run_mutable_checks(state).await,
            Self::RecoverIbcClient(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::CurrencyPairsChange(checked_action) => {
                checked_action.run_mutable_checks(state).await
            }
            Self::MarketsChange(checked_action) => checked_action.run_mutable_checks(state).await,
        }
        .map_err(|source| CheckedActionMutableCheckError {
            action_name: self.name(),
            source,
        })
    }

    #[expect(clippy::too_many_lines, reason = "we have a lot of action variants")]
    pub(crate) async fn pay_fees_and_execute<S: StateWrite>(
        &self,
        mut state: S,
        tx_signer: &[u8; ADDRESS_LENGTH],
        tx_id: &TransactionId,
        position_in_tx: u64,
    ) -> Result<(), CheckedActionExecutionError> {
        match self {
            Self::RollupDataSubmission(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await
                // Nothing to execute.
            }
            Self::Transfer(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::ValidatorUpdate(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::SudoAddressChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::IbcRelay(checked_action) => {
                state.ephemeral_put_ibc_context(*tx_id, position_in_tx);
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                if let Err(source) = checked_action.execute(&mut state).await {
                    // Determine whether to report this as a fatal error (pre-Blackburn) or not.
                    let is_fatal = state
                        .get_upgrade_change_info(&Blackburn::NAME, &AllowIbcRelayToFail::NAME)
                        .await
                        .map(|maybe_change_info| maybe_change_info.is_none())
                        .unwrap_or(true);
                    let error = if is_fatal {
                        CheckedActionExecutionError::execution(
                            checked_action.action().name(),
                            source,
                        )
                    } else {
                        CheckedActionExecutionError::non_fatal_execution(
                            checked_action.action().name(),
                            source,
                        )
                    };
                    return Err(error);
                }
                Ok(())
            }
            Self::IbcSudoChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::Ics20Withdrawal(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::IbcRelayerChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::FeeAssetChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::InitBridgeAccount(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::BridgeLock(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::BridgeUnlock(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::BridgeSudoChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::BridgeTransfer(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::FeeChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::RecoverIbcClient(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::CurrencyPairsChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
            Self::MarketsChange(checked_action) => {
                pay_fee(
                    checked_action.action(),
                    tx_signer,
                    position_in_tx,
                    &mut state,
                )
                .await?;
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source)
                })
            }
        }
    }

    pub(crate) fn asset_and_amount_to_transfer(&self) -> Option<(IbcPrefixed, u128)> {
        match self {
            CheckedAction::RollupDataSubmission(action) => action.transfer_asset_and_amount(),
            CheckedAction::Transfer(action) => action.transfer_asset_and_amount(),
            CheckedAction::ValidatorUpdate(action) => action.transfer_asset_and_amount(),
            CheckedAction::SudoAddressChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::IbcRelay(action) => action.transfer_asset_and_amount(),
            CheckedAction::IbcSudoChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::Ics20Withdrawal(action) => action.transfer_asset_and_amount(),
            CheckedAction::IbcRelayerChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::FeeAssetChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::InitBridgeAccount(action) => action.transfer_asset_and_amount(),
            CheckedAction::BridgeLock(action) => action.transfer_asset_and_amount(),
            CheckedAction::BridgeUnlock(action) => action.transfer_asset_and_amount(),
            CheckedAction::BridgeSudoChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::BridgeTransfer(action) => action.transfer_asset_and_amount(),
            CheckedAction::FeeChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::RecoverIbcClient(action) => action.transfer_asset_and_amount(),
            CheckedAction::CurrencyPairsChange(action) => action.transfer_asset_and_amount(),
            CheckedAction::MarketsChange(action) => action.transfer_asset_and_amount(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            CheckedAction::RollupDataSubmission(checked_action) => checked_action.action().name(),
            CheckedAction::Transfer(checked_action) => checked_action.action().name(),
            CheckedAction::ValidatorUpdate(checked_action) => checked_action.action().name(),
            CheckedAction::SudoAddressChange(checked_action) => checked_action.action().name(),
            CheckedAction::IbcRelay(checked_action) => checked_action.action().name(),
            CheckedAction::IbcSudoChange(checked_action) => checked_action.action().name(),
            CheckedAction::Ics20Withdrawal(checked_action) => checked_action.action().name(),
            CheckedAction::IbcRelayerChange(checked_action) => checked_action.action().name(),
            CheckedAction::FeeAssetChange(checked_action) => checked_action.action().name(),
            CheckedAction::InitBridgeAccount(checked_action) => checked_action.action().name(),
            CheckedAction::BridgeLock(checked_action) => checked_action.action().name(),
            CheckedAction::BridgeUnlock(checked_action) => checked_action.action().name(),
            CheckedAction::BridgeSudoChange(checked_action) => checked_action.action().name(),
            CheckedAction::BridgeTransfer(checked_action) => checked_action.action().name(),
            CheckedAction::FeeChange(checked_action) => checked_action.action().name(),
            CheckedAction::RecoverIbcClient(checked_action) => checked_action.action().name(),
            CheckedAction::CurrencyPairsChange(checked_action) => checked_action.action().name(),
            CheckedAction::MarketsChange(checked_action) => checked_action.action().name(),
        }
    }
}

#[instrument(skip_all, fields(action = %action.name()), err(level = Level::DEBUG))]
async fn pay_fee<'a, F, S>(
    action: &'a F,
    tx_signer: &[u8; ADDRESS_LENGTH],
    position_in_transaction: u64,
    mut state: S,
) -> Result<(), CheckedActionExecutionError>
where
    F: FeeHandler,
    S: StateWrite,
    FeeComponents<F>: TryFrom<StoredValue<'a>, Error = eyre::Report>,
{
    let Some((fee_asset, total_fee)) = super::utils::fee(action, &state).await? else {
        // If the action has no associated fee asset, there are no fees to pay.
        return Ok(());
    };

    state
        .add_fee_to_block_fees::<_, F>(fee_asset, total_fee, position_in_transaction)
        .map_err(|source| {
            CheckedActionFeeError::internal("failed adding fee to block fees", source)
        })?;
    state
        .decrease_balance(tx_signer, fee_asset, total_fee)
        .await
        .map_err(|source| {
            if source.downcast_ref::<InsufficientFunds>().is_some() {
                CheckedActionFeeError::InsufficientBalanceToPayFee {
                    account: *tx_signer,
                    asset: fee_asset.clone(),
                    amount: total_fee,
                }
                .into()
            } else {
                CheckedActionFeeError::internal(
                    "failed to decrease balance for fee payment",
                    source,
                )
                .into()
            }
        })
}

#[cfg(test)]
impl From<CheckedAction> for CheckedRollupDataSubmission {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::RollupDataSubmission(wrapped_action) = checked_action else {
            panic!("expected RollupDataSubmission");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedTransfer {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::Transfer(wrapped_action) = checked_action else {
            panic!("expected Transfer");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedValidatorUpdate {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::ValidatorUpdate(wrapped_action) = checked_action else {
            panic!("expected ValidatorUpdate");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedSudoAddressChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::SudoAddressChange(wrapped_action) = checked_action else {
            panic!("expected SudoAddressChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedIbcRelay {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::IbcRelay(wrapped_action) = checked_action else {
            panic!("expected IbcRelay");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedIbcSudoChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::IbcSudoChange(wrapped_action) = checked_action else {
            panic!("expected IbcSudoChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedIcs20Withdrawal {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::Ics20Withdrawal(wrapped_action) = checked_action else {
            panic!("expected Ics20Withdrawal");
        };
        *wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedIbcRelayerChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::IbcRelayerChange(wrapped_action) = checked_action else {
            panic!("expected IbcRelayerChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedFeeAssetChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::FeeAssetChange(wrapped_action) = checked_action else {
            panic!("expected FeeAssetChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedInitBridgeAccount {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::InitBridgeAccount(wrapped_action) = checked_action else {
            panic!("expected InitBridgeAccount");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedBridgeLock {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::BridgeLock(wrapped_action) = checked_action else {
            panic!("expected BridgeLock");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedBridgeUnlock {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::BridgeUnlock(wrapped_action) = checked_action else {
            panic!("expected BridgeUnlock");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedBridgeSudoChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::BridgeSudoChange(wrapped_action) = checked_action else {
            panic!("expected BridgeSudoChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedBridgeTransfer {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::BridgeTransfer(wrapped_action) = checked_action else {
            panic!("expected BridgeTransfer");
        };
        *wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedFeeChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::FeeChange(wrapped_action) = checked_action else {
            panic!("expected FeeChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedRecoverIbcClient {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::RecoverIbcClient(wrapped_action) = checked_action else {
            panic!("expected RecoverIbcClient");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedCurrencyPairsChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::CurrencyPairsChange(wrapped_action) = checked_action else {
            panic!("expected CurrencyPairsChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
impl From<CheckedAction> for CheckedMarketsChange {
    fn from(checked_action: CheckedAction) -> Self {
        let CheckedAction::MarketsChange(wrapped_action) = checked_action else {
            panic!("expected MarketsChange");
        };
        wrapped_action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        dummy_rollup_data_submission,
        Fixture,
    };

    #[tokio::test]
    async fn should_report_insufficient_funds() {
        let mut fixture = Fixture::default_initialized().await;
        let tx_signer = [20; ADDRESS_LENGTH];
        let action = dummy_rollup_data_submission();
        let expected_fees = super::super::utils::fee(&action, fixture.state())
            .await
            .expect("should get fees")
            .expect("fees should be `Some`");

        let error = pay_fee(&action, &tx_signer, 1, fixture.state_mut())
            .await
            .unwrap_err();

        match error {
            CheckedActionExecutionError::Fee(
                CheckedActionFeeError::InsufficientBalanceToPayFee {
                    account,
                    asset,
                    amount,
                },
            ) => {
                assert_eq!(account, tx_signer);
                assert_eq!(asset, *expected_fees.0);
                assert_eq!(amount, expected_fees.1);
            }
            _ => panic!("should be fee error, got {error:?}"),
        };
    }
}
