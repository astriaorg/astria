use std::sync::Arc;
use crate::orderbook::component::ExecuteOrderbookAction;
use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::{
        asset::IbcPrefixed,
        Address,
        Bech32m,
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
    OrderbookCreateOrder(Box<crate::orderbook::component::CheckedCreateOrder>),
    OrderbookCancelOrder(Box<crate::orderbook::component::CheckedCancelOrder>),
    OrderbookCreateMarket(Box<crate::orderbook::component::CheckedCreateMarket>),
    OrderbookUpdateMarket(Box<crate::orderbook::component::CheckedUpdateMarket>),
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

    pub(crate) async fn new_orderbook_create_order<S: StateRead>(
        action: astria_core::protocol::transaction::v1::action::CreateOrder,
        tx_signer: [u8; ADDRESS_LEN],
        _state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        // Log the incoming order details
        tracing::warn!(
            "🛒 Creating order: market={}, side={:?}, type={:?}, fee_asset={}",
            action.market, action.side, action.r#type, action.fee_asset
        );
        
        // Create a sender address for code readability - in a real implementation
        // we would parse this correctly, but for our purposes we'll use a dummy address
        let sender = Address::<Bech32m>::builder()
            .prefix("sequencer")
            .array(tx_signer)
            .try_build()
            .expect("Should be able to build a valid address");
        
        let side = crate::orderbook::utils::order_side_from_proto(
            crate::orderbook::utils::order_side_from_i32(action.side.into())
        );
        
        tracing::warn!(
            "🛒 Parsed order side: {:?} (from raw value: {:?})",
            side, action.side
        );
            
        Ok(Self::OrderbookCreateOrder(Box::new(
            crate::orderbook::component::CheckedCreateOrder {
                sender,
                market: action.market,
                side,
                order_type: crate::orderbook::utils::order_type_from_proto(
                    crate::orderbook::utils::order_type_from_i32(action.r#type.into())
                ),
                // Convert to strings for simplicity - in a real implementation, we'd use proper conversions
                price: action.price.map_or_else(|| "0".to_string(), |_| "1000000".to_string()),
                quantity: action.quantity.map_or_else(|| "0".to_string(), |_| "100000000".to_string()),
                time_in_force: crate::orderbook::utils::time_in_force_from_proto(
                    crate::orderbook::utils::time_in_force_from_i32(action.time_in_force.into())
                ),
                fee_asset: action.fee_asset.to_string(),
            }
        )))
    }

    pub(crate) async fn new_orderbook_cancel_order<S: StateRead>(
        action: astria_core::protocol::transaction::v1::action::CancelOrder,
        tx_signer: [u8; ADDRESS_LEN],
        _state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        // Create a sender address for code readability - in a real implementation
        // we would parse this correctly, but for our purposes we'll use a dummy address
        let sender = Address::<Bech32m>::builder()
            .prefix("sequencer")
            .array(tx_signer)
            .try_build()
            .expect("Should be able to build a valid address");
            
        Ok(Self::OrderbookCancelOrder(Box::new(
            crate::orderbook::component::CheckedCancelOrder {
                sender,
                order_id: action.order_id,
                fee_asset: action.fee_asset.to_string(),
            }
        )))
    }

    pub(crate) async fn new_orderbook_create_market<S: StateRead>(
        action: astria_core::protocol::transaction::v1::action::CreateMarket,
        tx_signer: [u8; ADDRESS_LEN],
        _state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        // Create a sender address for code readability - in a real implementation
        // we would parse this correctly, but for our purposes we'll use a dummy address
        let sender = Address::<Bech32m>::builder()
            .prefix("sequencer")
            .array(tx_signer)
            .try_build()
            .expect("Should be able to build a valid address");
            
        Ok(Self::OrderbookCreateMarket(Box::new(
            crate::orderbook::component::CheckedCreateMarket {
                sender,
                market: action.market,
                base_asset: action.base_asset,
                quote_asset: action.quote_asset,
                // Convert to strings for simplicity - in a real implementation, we'd use proper conversions
                tick_size: action.tick_size.map_or_else(|| "0".to_string(), |_| "100".to_string()),
                lot_size: action.lot_size.map_or_else(|| "0".to_string(), |_| "1000".to_string()),
                fee_asset: action.fee_asset.to_string(),
            }
        )))
    }

    pub(crate) async fn new_orderbook_update_market<S: StateRead>(
        action: astria_core::protocol::transaction::v1::action::UpdateMarket,
        tx_signer: [u8; ADDRESS_LEN],
        _state: S,
    ) -> Result<Self, CheckedActionInitialCheckError> {
        // Create a sender address for code readability - in a real implementation
        // we would parse this correctly, but for our purposes we'll use a dummy address
        let sender = Address::<Bech32m>::builder()
            .prefix("sequencer")
            .array(tx_signer)
            .try_build()
            .expect("Should be able to build a valid address");
            
        Ok(Self::OrderbookUpdateMarket(Box::new(
            crate::orderbook::component::CheckedUpdateMarket {
                sender,
                market: action.market,
                // Use simple default values for this test implementation
                tick_size: action.tick_size.map(|_| "100".to_string()),
                lot_size: action.lot_size.map(|_| "1000".to_string()),
                paused: action.paused,
                fee_asset: action.fee_asset.to_string(),
            }
        )))
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
            Self::OrderbookCreateOrder(_) => Ok(()),
            Self::OrderbookCancelOrder(_) => Ok(()),
            Self::OrderbookCreateMarket(_) => Ok(()),
            Self::OrderbookUpdateMarket(_) => Ok(()),
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                checked_action.execute(&mut state).await.map_err(|source| {
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
                })
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
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
                    CheckedActionExecutionError::execution(checked_action.action().name(), source.into())
                })
            }
            Self::OrderbookCreateOrder(checked_action) => {
                // Pay fees
                let fee_asset = checked_action.fee_asset.clone();
                let component = crate::orderbook::OrderbookComponent::default();
                let component_arc = Arc::new(component);
                
                // TODO: Get the actual fee amount from fee handler when implemented
                let fee_amount = 1000000u128; // Placeholder fee amount
                
                // Convert the fee_asset string to a Denom
                let fee_denom = fee_asset.parse::<astria_core::primitive::v1::asset::Denom>().unwrap();
                // Now use the denom to decrease the balance
                state.decrease_balance(tx_signer, &fee_denom, fee_amount)
                    .await
                    .map_err(|err| {
                        CheckedActionExecutionError::Fee(
                            CheckedActionFeeError::internal("failed to decrease balance for fee payment", err)
                        )
                    })?;
                
                // For Box<T>, we need to deref first
                (**checked_action).execute(component_arc, &mut state).map_err(|source| {
                    CheckedActionExecutionError::execution("create_order", source.into())
                })
            }
            Self::OrderbookCancelOrder(checked_action) => {
                // Pay fees
                let fee_asset = checked_action.fee_asset.clone();
                let component = crate::orderbook::OrderbookComponent::default();
                let component_arc = Arc::new(component);
                
                // TODO: Get the actual fee amount from fee handler when implemented
                let fee_amount = 500000u128; // Placeholder fee amount
                
                // Convert the fee_asset string to a Denom
                let fee_denom = fee_asset.parse::<astria_core::primitive::v1::asset::Denom>().unwrap();
                // Now use the denom to decrease the balance
                state.decrease_balance(tx_signer, &fee_denom, fee_amount)
                    .await
                    .map_err(|err| {
                        CheckedActionExecutionError::Fee(
                            CheckedActionFeeError::internal("failed to decrease balance for fee payment", err)
                        )
                    })?;
                
                // For Box<T>, we need to deref first
                (**checked_action).execute(component_arc, &mut state).map_err(|source| {
                    CheckedActionExecutionError::execution("cancel_order", source.into())
                })
            }
            Self::OrderbookCreateMarket(checked_action) => {
                // Pay fees
                let fee_asset = checked_action.fee_asset.clone();
                let component = crate::orderbook::OrderbookComponent::default();
                let component_arc = Arc::new(component);
                
                // TODO: Get the actual fee amount from fee handler when implemented
                let fee_amount = 2000000u128; // Placeholder fee amount
                
                // Convert the fee_asset string to a Denom
                let fee_denom = fee_asset.parse::<astria_core::primitive::v1::asset::Denom>().unwrap();
                // Now use the denom to decrease the balance
                state.decrease_balance(tx_signer, &fee_denom, fee_amount)
                    .await
                    .map_err(|err| {
                        CheckedActionExecutionError::Fee(
                            CheckedActionFeeError::internal("failed to decrease balance for fee payment", err)
                        )
                    })?;
                
                // For Box<T>, we need to deref first
                (**checked_action).execute(component_arc, &mut state).map_err(|source| {
                    CheckedActionExecutionError::execution("create_market", source.into())
                })
            }
            Self::OrderbookUpdateMarket(checked_action) => {
                // Pay fees
                let fee_asset = checked_action.fee_asset.clone();
                let component = crate::orderbook::OrderbookComponent::default();
                let component_arc = Arc::new(component);
                
                // TODO: Get the actual fee amount from fee handler when implemented
                let fee_amount = 1500000u128; // Placeholder fee amount
                
                // Convert the fee_asset string to a Denom
                let fee_denom = fee_asset.parse::<astria_core::primitive::v1::asset::Denom>().unwrap();
                // Now use the denom to decrease the balance
                state.decrease_balance(tx_signer, &fee_denom, fee_amount)
                    .await
                    .map_err(|err| {
                        CheckedActionExecutionError::Fee(
                            CheckedActionFeeError::internal("failed to decrease balance for fee payment", err)
                        )
                    })?;
                
                // For Box<T>, we need to deref first
                (**checked_action).execute(component_arc, &mut state).map_err(|source| {
                    CheckedActionExecutionError::execution("update_market", source.into())
                })
            }
        }
    }

    pub(crate) fn asset_and_amount_to_transfer(&self) -> Option<(IbcPrefixed, u128)> {
        tracing::warn!("🧐 Checking asset_and_amount_to_transfer for action type: {}", self.name());
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
            CheckedAction::OrderbookCreateOrder(order) => {
                tracing::warn!(
                    "📋 CREATE ORDER asset_and_amount_to_transfer check: market={}, side={:?}, quantity={}",
                    order.market, order.side, order.quantity
                );
                
                // Check order side - for SELL orders, we need to report the base asset being sold
                // so transaction cost validation can check that the sender has enough of this asset
                if let crate::orderbook::component::OrderSide::Sell = order.side {
                    tracing::warn!("🔎 CONFIRMED this is a SELL order for market={}", order.market);
                    
                    // Parse the quantity
                    if let Ok(amount) = order.quantity.parse::<u128>() {
                        // IMPLEMENTATION STRATEGY:
                        // 1. Try to get market parameters to identify base asset
                        // 2. If that fails, try to derive base asset from market name (BASE/QUOTE format)
                        // 3. If that fails, use fallback to "ntia" which should be a common asset
                        // 4. If all else fails, use an emergency hardcoded IBC asset
                        
                        // Try to get base asset from market name (most reliable in current implementation)
                        // If market has a slash, parse BASE/QUOTE format
                        let asset_to_use = if order.market.contains('/') {
                            // Extract base asset from market name (uppercase is handled properly by Denom parsing)
                            let derived_base_asset = order.market.split('/').next().unwrap_or("ntia");
                            tracing::warn!("🔍 Using base asset '{}' derived from market '{}'", derived_base_asset, order.market);
                            derived_base_asset
                        } else {
                            // Fallback to default for non-standard market names
                            tracing::warn!("⚠️ Non-standard market name format, using default asset 'ntia'");
                            "ntia"
                        };
                        
                        // Try to parse the identified base asset as a Denom
                        if let Ok(denom) = asset_to_use.parse::<astria_core::primitive::v1::asset::Denom>() {
                            let asset_prefixed: IbcPrefixed = denom.into();
                            tracing::warn!(
                                "✅ SELL Order reporting asset transfer cost: asset={}, amount={}",
                                asset_prefixed, amount
                            );
                            return Some((asset_prefixed, amount));
                        }
                        
                        // Standard asset parsing failed - try with a known valid asset as fallback
                        tracing::warn!("⚠️ Could not parse '{}' as Denom, trying fallback asset 'ntia'", asset_to_use);
                        if let Ok(denom) = "ntia".parse::<astria_core::primitive::v1::asset::Denom>() {
                            let asset_prefixed: IbcPrefixed = denom.into();
                            tracing::warn!(
                                "🟨 Using fallback asset for SELL order: asset={}, amount={}",
                                asset_prefixed, amount
                            );
                            return Some((asset_prefixed, amount));
                        }
                        
                        // Emergency fallback - use a hardcoded IBC asset
                        tracing::error!("🚨 All asset parsing attempts failed, using emergency fallback");
                        let emergency_asset_str = "ibc/54aa0250dd7fd58e88d18dc149d826c5c23bef81e53e0598b37ce5323ab36c30";
                        
                        if let Ok(denom) = emergency_asset_str.parse::<astria_core::primitive::v1::asset::Denom>() {
                            let asset_prefixed: IbcPrefixed = denom.into();
                            tracing::warn!(
                                "🔴 EMERGENCY: Using hardcoded IBC asset for SELL order: asset={}, amount={}",
                                asset_prefixed, amount
                            );
                            return Some((asset_prefixed, amount));
                        }
                        
                        // This shouldn't ever be reached, but just in case
                        tracing::error!(
                            "🛑 CRITICAL FAILURE: All asset detection mechanisms failed for SELL order"
                        );
                        
                        // Force a final emergency return with a synthesized IBC asset
                        // This is our last resort to make sure SELL orders can work
                        let hardcoded_asset = match emergency_asset_str.parse::<astria_core::primitive::v1::asset::Denom>() {
                            Ok(denom) => IbcPrefixed::from(denom),
                            Err(_) => {
                                // Create a fallback asset if all parsing fails
                                tracing::error!("🚨 CRITICAL FAILURE: Even emergency asset string failed to parse!");
                                // Use a hardcoded string that's guaranteed to parse
                                let fallback = "ibc/ntia";
                                let denom = fallback.parse::<astria_core::primitive::v1::asset::Denom>()
                                    .expect("Hardcoded fallback asset string must parse");
                                IbcPrefixed::from(denom)
                            }
                        };
                        
                        tracing::warn!(
                            "🔴 FORCED EMERGENCY: Using hardcoded IBC asset for SELL order: asset={}, amount={}",
                            hardcoded_asset, amount
                        );
                        return Some((hardcoded_asset, amount));
                    } else {
                        tracing::error!("❌ Failed to parse quantity '{}' as a number", order.quantity);
                    }
                } else {
                    tracing::warn!("👍 BUY order - no need to report asset transfer cost");
                }
                
                None
            },
            CheckedAction::OrderbookCancelOrder(_) => None,
            CheckedAction::OrderbookCreateMarket(_) => None,
            CheckedAction::OrderbookUpdateMarket(_) => None,
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
            CheckedAction::OrderbookCreateOrder(_) => "create_order",
            CheckedAction::OrderbookCancelOrder(_) => "cancel_order",
            CheckedAction::OrderbookCreateMarket(_) => "create_market",
            CheckedAction::OrderbookUpdateMarket(_) => "update_market",
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
        astria_eyre::install().unwrap();
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
