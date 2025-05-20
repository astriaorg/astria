use std::collections::HashMap;

use astria_core::{
    crypto::{
        VerificationKey,
        ADDRESS_LENGTH,
    },
    generated::astria::protocol::transaction::v1 as raw,
    primitive::v1::{
        asset::IbcPrefixed,
        RollupId,
        TransactionId,
    },
    protocol::transaction::v1::{
        Action,
        Group,
        Transaction,
        TransactionParams,
        TransactionParts,
    },
    Protobuf as _,
};
use bytes::Bytes;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::future::try_join_all;
use prost::Message as _;
use sha2::Digest as _;
use tracing::{
    instrument,
    Level,
};

pub(crate) use self::error::{
    CheckedTransactionExecutionError,
    CheckedTransactionInitialCheckError,
};
use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt as _,
    },
    app::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    checked_actions::{
        utils::total_fees,
        ActionRef,
        CheckedAction,
        CheckedActionFeeError,
        CheckedActionMutableCheckError,
    },
    orderbook::component::{
        CheckedCreateOrder,
        CheckedCancelOrder,
        CheckedCreateMarket,
        CheckedUpdateMarket,
    },
};

mod error;
#[cfg(test)]
mod tests;

const MAX_TX_BYTES: usize = 256_000;

/// A transaction that has undergone initial validity checks, and that can be rechecked before
/// execution as often as required.
///
/// Nonce checks and account balance checks are excluded from these, as the `Mempool` ensures
/// transactions put forward for execution have the correct nonces and sufficient balances.
///
/// Checks with immutable outcomes are not rechecked.
///
/// This type is used throughout the sequencer rather than the unchecked [`Transaction`] to ensure
/// appropriate checks have always been run.
#[derive(Debug)]
pub(crate) struct CheckedTransaction {
    tx_id: TransactionId,
    actions: Vec<CheckedAction>,
    group: Group,
    params: TransactionParams,
    verification_key: VerificationKey,
    tx_bytes: Bytes,
}

impl CheckedTransaction {
    /// Returns a new `CheckedTransaction` by parsing the given `tx_bytes` into a [`Transaction`],
    /// validating it, and converting to a `CheckedTransaction`.
    ///
    /// NOTE: To construct a `CheckedTransaction` for tests, it is generally simplest to use
    /// [`Fixture::checked_tx_builder`].
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(crate) async fn new<S: StateRead>(
        tx_bytes: Bytes,
        state: &S,
    ) -> Result<Self, CheckedTransactionInitialCheckError> {
        let tx_len = tx_bytes.len();
        if tx_len > MAX_TX_BYTES {
            return Err(CheckedTransactionInitialCheckError::TooLarge {
                max_len: MAX_TX_BYTES,
                tx_len,
            });
        }

        let raw_tx = raw::Transaction::decode(tx_bytes.clone())
            .map_err(CheckedTransactionInitialCheckError::Decode)?;
        
        // Log the raw transaction details
        tracing::warn!(
            "📦 Raw transaction being processed: tx_bytes_len={}",
            tx_bytes.len()
        );
        
        let tx = Transaction::try_from_raw(raw_tx)
            .map_err(CheckedTransactionInitialCheckError::Convert)?;
        
        // Log transaction details after conversion
        tracing::warn!(
            "📦 Transaction parsed: chain_id={}, nonce={}",
            tx.chain_id(), tx.nonce()
        );

        let current_nonce =
            state
                .get_account_nonce(tx.address_bytes())
                .await
                .map_err(|source| {
                    CheckedTransactionInitialCheckError::internal(
                        "failed to read nonce from storage",
                        source,
                    )
                })?;
        let tx_nonce = tx.nonce();
        if tx_nonce < current_nonce {
            return Err(CheckedTransactionInitialCheckError::InvalidNonce {
                current_nonce,
                tx_nonce,
            });
        };

        let tx_id = TransactionId::new(sha2::Sha256::digest(&tx_bytes).into());
        let tx_chain_id = tx.chain_id().to_string();

        let TransactionParts {
            actions: unchecked_actions,
            group,
            params,
            verification_key,
        } = tx.into_parts();
        let tx_signer = *verification_key.address_bytes();
        
        // Log the unchecked actions
        tracing::warn!(
            "📦 Transaction contains {} actions, group={:?}", 
            unchecked_actions.len(), group
        );
        
        // Log details about each action
        for (idx, action) in unchecked_actions.iter().enumerate() {
            let action_name = match action {
                Action::RollupDataSubmission(_) => "RollupDataSubmission",
                Action::Transfer(_) => "Transfer",
                Action::ValidatorUpdate(_) => "ValidatorUpdate",
                Action::SudoAddressChange(_) => "SudoAddressChange",
                Action::Ibc(_) => "Ibc",
                Action::IbcSudoChange(_) => "IbcSudoChange",
                Action::Ics20Withdrawal(_) => "Ics20Withdrawal",
                Action::IbcRelayerChange(_) => "IbcRelayerChange",
                Action::FeeAssetChange(_) => "FeeAssetChange",
                Action::InitBridgeAccount(_) => "InitBridgeAccount",
                Action::BridgeLock(_) => "BridgeLock",
                Action::BridgeUnlock(_) => "BridgeUnlock",
                Action::BridgeSudoChange(_) => "BridgeSudoChange",
                Action::BridgeTransfer(_) => "BridgeTransfer",
                Action::FeeChange(_) => "FeeChange",
                Action::RecoverIbcClient(_) => "RecoverIbcClient",
                Action::CurrencyPairsChange(_) => "CurrencyPairsChange",
                Action::MarketsChange(_) => "MarketsChange",
                Action::CreateOrder(order) => {
                    tracing::warn!(
                        "🔍 FOUND ORDER ACTION: idx={}, market={}, side={:?}, order_type={:?}",
                        idx, order.market, order.side, order.r#type
                    );
                    "CreateOrder"
                },
                Action::CancelOrder(_) => "CancelOrder",
                Action::CreateMarket(_) => "CreateMarket",
                Action::UpdateMarket(_) => "UpdateMarket",
            };
            
            if action_name != "CreateOrder" {  // Already logged above
                tracing::warn!("📑 Action at idx={}: {}", idx, action_name);
            }
        }
        
        let checked_actions =
            match convert_actions(unchecked_actions, tx_signer, tx_id, state).await {
                Ok(checked_actions) => checked_actions,
                Err(error) => {
                    tracing::error!("❌ Failed to convert actions: {:?}", error);
                    return Err(error);
                }
            };

        let chain_id = state.get_chain_id().await.map_err(|source| {
            CheckedTransactionInitialCheckError::internal(
                "failed to get chain id from storage",
                source,
            )
        })?;
        if tx_chain_id != chain_id.as_str() {
            return Err(CheckedTransactionInitialCheckError::ChainIdMismatch {
                expected: chain_id.as_str().to_string(),
                tx_chain_id,
            });
        }

        Ok(Self {
            tx_id,
            actions: checked_actions,
            group,
            params,
            verification_key,
            tx_bytes,
        })
    }

    pub(crate) fn id(&self) -> &TransactionId {
        &self.tx_id
    }

    #[must_use]
    pub(crate) fn checked_actions(&self) -> &[CheckedAction] {
        &self.actions
    }

    pub(crate) fn group(&self) -> Group {
        self.group
    }

    pub(crate) fn nonce(&self) -> u32 {
        self.params.nonce()
    }

    pub(crate) fn chain_id(&self) -> &str {
        self.params.chain_id()
    }

    pub(crate) fn verification_key(&self) -> &VerificationKey {
        &self.verification_key
    }

    /// Returns the bytes of the encoded `Transaction` from which this `CheckedTransaction` is
    /// constructed.
    pub(crate) fn encoded_bytes(&self) -> &Bytes {
        &self.tx_bytes
    }

    /// Returns an iterator over the rollup ID and data bytes of all `RollupDataSubmission`s in this
    /// transaction's actions, in the order in which they occur in the transaction.
    pub(crate) fn rollup_data_bytes(&self) -> impl Iterator<Item = (&RollupId, &Bytes)> {
        self.actions.iter().filter_map(|checked_action| {
            if let CheckedAction::RollupDataSubmission(rollup_submission) = checked_action {
                Some((
                    &rollup_submission.action().rollup_id,
                    &rollup_submission.action().data,
                ))
            } else {
                None
            }
        })
    }

    /// Returns the total costs involved in executing this transaction, i.e. all of the fees and
    /// outbound transfers of all actions in this transaction.
    pub(crate) async fn total_costs<S: StateRead>(
        &self,
        state: &S,
    ) -> Result<HashMap<IbcPrefixed, u128>, CheckedActionFeeError> {
        // Get the base fee costs
        let mut cost_by_asset = total_fees(self.actions.iter().map(ActionRef::from), state).await?;
        
        // Log the fees calculated from actions
        tracing::warn!(
            "💰 Transaction fees (before asset transfers): tx_id={}, fees={:?}",
            self.tx_id, cost_by_asset
        );

        // Add the asset transfer costs
        for (index, action) in self.actions.iter().enumerate() {
            // Log the action type for debugging - use a match to safely get the type
            let action_type = match action {
                CheckedAction::OrderbookCreateOrder(_) => "create_order",
                CheckedAction::OrderbookCancelOrder(_) => "cancel_order",
                CheckedAction::OrderbookCreateMarket(_) => "create_market",
                CheckedAction::OrderbookUpdateMarket(_) => "update_market",
                CheckedAction::RollupDataSubmission(_) => "rollup_data_submission",
                CheckedAction::Transfer(_) => "transfer",
                CheckedAction::ValidatorUpdate(_) => "validator_update",
                CheckedAction::SudoAddressChange(_) => "sudo_address_change",
                CheckedAction::IbcRelay(_) => "ibc_relay",
                CheckedAction::IbcSudoChange(_) => "ibc_sudo_change",
                CheckedAction::Ics20Withdrawal(_) => "ics20_withdrawal",
                CheckedAction::IbcRelayerChange(_) => "ibc_relayer_change",
                CheckedAction::FeeAssetChange(_) => "fee_asset_change",
                CheckedAction::InitBridgeAccount(_) => "init_bridge_account",
                CheckedAction::BridgeLock(_) => "bridge_lock",
                CheckedAction::BridgeUnlock(_) => "bridge_unlock",
                CheckedAction::BridgeSudoChange(_) => "bridge_sudo_change",
                CheckedAction::BridgeTransfer(_) => "bridge_transfer",
                CheckedAction::FeeChange(_) => "fee_change",
                CheckedAction::RecoverIbcClient(_) => "recover_ibc_client",
                CheckedAction::CurrencyPairsChange(_) => "currency_pairs_change",
                CheckedAction::MarketsChange(_) => "markets_change",
            };

            tracing::warn!(
                "🔍 Processing action {}/{}: type={}",
                index + 1, self.actions.len(), action_type
            );
            
            // Special handling for orderbook actions
            match action {
                CheckedAction::OrderbookCreateOrder(order) => {
                    tracing::warn!(
                        "🔖 Order details: market={}, side={:?}, quantity={}",
                        order.market, order.side, order.quantity
                    );
                },
                _ => {}
            };
            // Get special details for CreateOrder actions
            let mut is_sell_order = false;
            let mut market_name = String::new();
            if let CheckedAction::OrderbookCreateOrder(ref order) = action {
                if let crate::orderbook::component::OrderSide::Sell = order.side {
                    is_sell_order = true;
                    market_name = order.market.clone();
                    tracing::warn!(
                        "🔍 Processing SELL order in market '{}', checking asset cost...",
                        market_name
                    );
                }
            }
            
            // Get and validate asset transfer costs
            let asset_transfer_result = action.asset_and_amount_to_transfer();
            
            if is_sell_order {
                if let Some((asset, amount)) = &asset_transfer_result {
                    tracing::warn!(
                        "✅ SELL order successfully reported asset cost: asset={}, amount={}, market={}",
                        asset, amount, market_name
                    );
                } else {
                    tracing::error!(
                        "❌ SELL order failed to report asset cost! This is a critical issue, market={}",
                        market_name
                    );
                }
            } else {
                tracing::warn!(
                    "🧐 Asset transfer for action {}: cost_reported={}", 
                    action_type, asset_transfer_result.is_some()
                );
            }
            
            if let Some((asset, amount)) = asset_transfer_result {
                tracing::warn!(
                    "💰 Adding asset transfer cost to transaction budget: tx_id={}, asset={}, amount={}",
                    self.tx_id, asset, amount
                );
                
                // Add to cost accounting
                cost_by_asset
                    .entry(asset.clone())
                    .and_modify(|amt| {
                        let new_amt = amt.saturating_add(amount);
                        tracing::warn!(
                            "💰 Updated transaction cost: asset={}, previous_total={}, additional={}, new_total={}",
                            asset, *amt, amount, new_amt
                        );
                        *amt = new_amt;
                    })
                    .or_insert_with(|| {
                        tracing::warn!(
                            "💰 New asset in transaction budget: asset={}, amount={}",
                            asset, amount
                        );
                        amount
                    });
            } else {
                if is_sell_order {
                    tracing::error!(
                        "❗ SELL order did not report asset transfer cost - this indicates a serious issue!"
                    );
                } else {
                    tracing::warn!(
                        "ℹ️ No asset transfer reported for action type: {} (this is normal for non-SELL orders)",
                        action_type
                    );
                }
            }
        }
        
        // Log the final costs
        tracing::warn!(
            "💰 Total transaction costs: tx_id={}, total_costs={:?}",
            self.tx_id, cost_by_asset
        );

        Ok(cost_by_asset)
    }

    /// Re-runs checks that passed during construction of the `CheckedTransaction`, but that might
    /// now fail due to changes in the global state.
    ///
    /// NOTE: excludes nonce and balance checks.
    #[expect(unused, reason = "will be used when CheckTx_Recheck handled properly")]
    pub(crate) async fn run_mutable_checks<S: StateRead>(
        &self,
        state: S,
    ) -> Result<(), CheckedActionMutableCheckError> {
        for action in &self.actions {
            action.run_mutable_checks(&state).await?;
        }
        Ok(())
    }

    /// Executes the actions in this transaction.
    ///
    /// Returns an error if the current nonce for the transaction's signer in `state` is different
    /// to this transaction's nonce. Also returns an error if any action fails execution, or the
    /// signer cannot pay the required execution costs.
    pub(super) async fn execute<S: StateWrite>(
        &self,
        mut state: S,
    ) -> Result<(), CheckedTransactionExecutionError> {
        // Nonce should be equal to the number of executed transactions before this tx.
        // First tx has nonce 0.
        let current_nonce = state
            .get_account_nonce(self.address_bytes())
            .await
            .map_err(|source| {
                CheckedTransactionExecutionError::internal(
                    "failed to read nonce from storage",
                    source,
                )
            })?;
        let tx_nonce = self.params.nonce();
        if current_nonce != tx_nonce {
            return Err(CheckedTransactionExecutionError::InvalidNonce {
                expected: current_nonce,
                tx_nonce,
            });
        };

        if state
            .get_bridge_account_rollup_id(self)
            .await
            .map_err(|source| {
                CheckedTransactionExecutionError::internal(
                    "failed to read bridge account rollup id from storage",
                    source,
                )
            })?
            .is_some()
        {
            state
                .put_last_transaction_id_for_bridge_account(self, self.tx_id)
                .map_err(|source| {
                    CheckedTransactionExecutionError::internal(
                        "failed to write last transaction id to storage",
                        source,
                    )
                })?;
        }

        let next_nonce = current_nonce
            .checked_add(1)
            .ok_or(CheckedTransactionExecutionError::NonceOverflowed)?;
        state
            .put_account_nonce(self, next_nonce)
            .map_err(|source| {
                CheckedTransactionExecutionError::internal("failed updating nonce", source)
            })?;

        let tx_signer = *self.verification_key.address_bytes();
        for (index, action) in self.actions.iter().enumerate() {
            let index = u64::try_from(index)
                .map_err(|_| CheckedTransactionExecutionError::ActionIndexOverflowed)?;
            action
                .pay_fees_and_execute(&mut state, &tx_signer, &self.tx_id, index)
                .await?;
        }
        Ok(())
    }
}

impl AddressBytes for CheckedTransaction {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        self.verification_key.address_bytes()
    }
}

async fn convert_actions<S: StateRead>(
    unchecked_actions: Vec<Action>,
    tx_signer: [u8; ADDRESS_LENGTH],
    tx_id: TransactionId,
    state: &S,
) -> Result<Vec<CheckedAction>, CheckedTransactionInitialCheckError> {
    tracing::warn!(
        "🔄 Converting {} raw actions to checked actions", 
        unchecked_actions.len()
    );
    let actions_futures =
        unchecked_actions
            .into_iter()
            .enumerate()
            .map(|(index, unchecked_action)| async move {
                let action_type = match &unchecked_action {
                    Action::RollupDataSubmission(_) => "RollupDataSubmission",
                    Action::Transfer(_) => "Transfer",
                    Action::ValidatorUpdate(_) => "ValidatorUpdate",
                    Action::SudoAddressChange(_) => "SudoAddressChange",
                    Action::Ibc(_) => "Ibc",
                    Action::IbcSudoChange(_) => "IbcSudoChange",
                    Action::Ics20Withdrawal(_) => "Ics20Withdrawal",
                    Action::IbcRelayerChange(_) => "IbcRelayerChange",
                    Action::FeeAssetChange(_) => "FeeAssetChange",
                    Action::InitBridgeAccount(_) => "InitBridgeAccount",
                    Action::BridgeLock(_) => "BridgeLock",
                    Action::BridgeUnlock(_) => "BridgeUnlock",
                    Action::BridgeSudoChange(_) => "BridgeSudoChange",
                    Action::BridgeTransfer(_) => "BridgeTransfer",
                    Action::FeeChange(_) => "FeeChange",
                    Action::RecoverIbcClient(_) => "RecoverIbcClient",
                    Action::CurrencyPairsChange(_) => "CurrencyPairsChange",
                    Action::MarketsChange(_) => "MarketsChange",
                    Action::CreateOrder(_) => "CreateOrder",
                    Action::CancelOrder(_) => "CancelOrder",
                    Action::CreateMarket(_) => "CreateMarket",
                    Action::UpdateMarket(_) => "UpdateMarket",
                };
                
                tracing::warn!(
                    "🔍 Processing unchecked action at idx={}: type={}",
                    index, action_type
                );
                
                let result = match unchecked_action {
                    Action::RollupDataSubmission(action) => {
                        CheckedAction::new_rollup_data_submission(action)
                    }
                    Action::Transfer(action) => {
                        CheckedAction::new_transfer(action, tx_signer, state).await
                    }
                    Action::ValidatorUpdate(action) => {
                        CheckedAction::new_validator_update(action, tx_signer, state).await
                    }
                    Action::SudoAddressChange(action) => {
                        CheckedAction::new_sudo_address_change(action, tx_signer, state).await
                    }
                    Action::Ibc(action) => {
                        CheckedAction::new_ibc_relay(action, tx_signer, state).await
                    }
                    Action::IbcSudoChange(action) => {
                        CheckedAction::new_ibc_sudo_change(action, tx_signer, state).await
                    }
                    Action::Ics20Withdrawal(action) => {
                        CheckedAction::new_ics20_withdrawal(action, tx_signer, state).await
                    }
                    Action::IbcRelayerChange(action) => {
                        CheckedAction::new_ibc_relayer_change(action, tx_signer, state).await
                    }
                    Action::FeeAssetChange(action) => {
                        CheckedAction::new_fee_asset_change(action, tx_signer, state).await
                    }
                    Action::InitBridgeAccount(action) => {
                        CheckedAction::new_init_bridge_account(action, tx_signer, state).await
                    }
                    Action::BridgeLock(action) => {
                        let position_in_tx = u64::try_from(index)
                            .expect("there should be less than `u64::MAX` actions in tx");
                        CheckedAction::new_bridge_lock(
                            action,
                            tx_signer,
                            tx_id,
                            position_in_tx,
                            state,
                        )
                        .await
                    }
                    Action::BridgeUnlock(action) => {
                        CheckedAction::new_bridge_unlock(action, tx_signer, state).await
                    }
                    Action::BridgeSudoChange(action) => {
                        CheckedAction::new_bridge_sudo_change(action, tx_signer, state).await
                    }
                    Action::BridgeTransfer(action) => {
                        let position_in_tx = u64::try_from(index)
                            .expect("there should be less than `u64::MAX` actions in tx");
                        CheckedAction::new_bridge_transfer(
                            action,
                            tx_signer,
                            tx_id,
                            position_in_tx,
                            state,
                        )
                        .await
                    }
                    Action::FeeChange(action) => {
                        CheckedAction::new_fee_change(action, tx_signer, state).await
                    }
                    Action::RecoverIbcClient(action) => {
                        CheckedAction::new_recover_ibc_client(action, tx_signer, state).await
                    }
                    Action::CurrencyPairsChange(action) => {
                        CheckedAction::new_currency_pairs_change(action, tx_signer, state).await
                    }
                    Action::MarketsChange(action) => {
                        CheckedAction::new_markets_change(action, tx_signer, state).await
                    }
                    Action::CreateOrder(action) => {
                        tracing::warn!(
                            "⭐ Processing CreateOrder action: market={}, side={:?}",
                            action.market, action.side
                        );
                        CheckedAction::new_orderbook_create_order(action, tx_signer, state).await
                    }
                    Action::CancelOrder(action) => {
                        CheckedAction::new_orderbook_cancel_order(action, tx_signer, state).await
                    }
                    Action::CreateMarket(action) => {
                        CheckedAction::new_orderbook_create_market(action, tx_signer, state).await
                    }
                    Action::UpdateMarket(action) => {
                        CheckedAction::new_orderbook_update_market(action, tx_signer, state).await
                    }
                };
                
                if let Err(ref err) = result {
                    tracing::error!(
                        "❌ Failed to create checked action for {}: {:?}",
                        action_type, err
                    );
                } else {
                    tracing::warn!(
                        "✅ Successfully created checked action for {}",
                        action_type
                    );
                }
                
                result
            });

    try_join_all(actions_futures)
        .await
        .map_err(CheckedTransactionInitialCheckError::CheckedAction)
}
