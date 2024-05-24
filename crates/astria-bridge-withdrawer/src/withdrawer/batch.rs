use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use ethers::types::{
    TxHash,
    U64,
};
use serde::{
    Deserialize,
    Serialize,
};

use super::ethereum::astria_withdrawer::{
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};

// const FEE_ASSET_ID: &str = "fee";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WithdrawalEvent {
    Sequencer(SequencerWithdrawalFilter),
    Ics20(Ics20WithdrawalFilter),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventWithMetadata {
    event: WithdrawalEvent,
    /// The block in which the log was emitted
    block_number: U64,
    /// The transaction hash in which the log was emitted
    transaction_hash: TxHash,
}

pub(crate) fn event_to_action(
    event_with_metadata: EventWithMetadata,
    fee_asset_id: asset::Id,
) -> eyre::Result<Action> {
    let action = match event_with_metadata.event {
        WithdrawalEvent::Sequencer(event) => event_to_bridge_unlock(
            event,
            event_with_metadata.block_number,
            event_with_metadata.transaction_hash,
            fee_asset_id,
        )
        .wrap_err("failed to convert sequencer withdrawal event to action")?,
        WithdrawalEvent::Ics20(event) => event_to_ics20_withdrawal(
            event,
            event_with_metadata.block_number,
            event_with_metadata.transaction_hash,
            fee_asset_id,
        )
        .wrap_err("failed to convert ics20 withdrawal event to action")?,
    };
    Ok(action)
}

#[derive(Debug, Serialize, Deserialize)]
struct BridgeUnlockMemo {
    block_number: U64,
    transaction_hash: TxHash,
}

fn event_to_bridge_unlock(
    event: SequencerWithdrawalFilter,
    block_number: U64,
    transaction_hash: TxHash,
    fee_asset_id: asset::Id,
) -> eyre::Result<Action> {
    let memo = BridgeUnlockMemo {
        block_number,
        transaction_hash,
    };
    let action = BridgeUnlockAction {
        to: event.sender.to_fixed_bytes().into(),
        amount: event.amount.as_u128(),
        memo: serde_json::to_vec(&memo)?,
        fee_asset_id,
    };
    Ok(Action::BridgeUnlock(action))
}

fn event_to_ics20_withdrawal(
    event: Ics20WithdrawalFilter,
    block_number: U64,
    transaction_hash: TxHash,
    fee_asset_id: asset::Id,
) -> eyre::Result<Action> {
    let action = Ics20Withdrawal {
        amount: event.amount.as_u128(),
        memo: event.memo.to_vec(), // TODO:
        fee_asset_id,
    };
    Ok(Action::Ics20Withdrawal(action))
}

pub(crate) struct Batch {
    /// The withdrawal payloads
    pub(crate) actions: Vec<Action>,
    /// The corresponding rollup block height
    pub(crate) rollup_height: u64,
}

#[cfg(test)]
mod tests {
    #[test]
    fn event_to_bridge_unlock() {
        // let event_with_meta = EventWithMetadata {
        //    event: WithdrawalFilter {
        //        sender: wallet.address(),
        //        amount: value,
        //        memo: b"nootwashere".into(),
        //    },
        //    block_number: receipt.block_number.unwrap(),
        //    transaction_hash: receipt.transaction_hash,
        //};
        // let action = event_to_action(event_with_meta);
        //
        // assert!(matches!(
        //    action,
        //    Action::BridgeUnlock(BridgeLockAction { .. })
        //))
    }
}
