use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1alpha1::{
        action::BridgeUnlockAction,
        Action,
    },
};
use astria_eyre::eyre;
use ethers::types::{
    TxHash,
    U64,
};

use super::ethereum::astria_withdrawer::WithdrawalFilter;

const FEE_ASSET_ID: &str = "fee";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventWithMetadata {
    pub(crate) event: WithdrawalFilter,
    /// The block in which the log was emitted
    pub(crate) block_number: U64,
    /// The transaction hash in which the log was emitted
    pub(crate) transaction_hash: TxHash,
}

pub(crate) fn event_to_action(
    event_with_metadata: EventWithMetadata,
    fee_asset_id: asset::Id,
) -> eyre::Result<Action> {
    let action = BridgeUnlockAction {
        to: event_with_metadata.event.sender.to_fixed_bytes().into(),
        amount: event_with_metadata.event.amount.as_u128(),
        memo: event_with_metadata.event.memo.to_vec(), // TODO: store block number and tx hash here
        fee_asset_id,
    };
    Ok(Action::BridgeUnlock(action))
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
