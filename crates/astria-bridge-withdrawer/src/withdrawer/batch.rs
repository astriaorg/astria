use astria_core::{
    primitive::v1::{
        asset,
        asset::Denom,
        Address,
    },
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
use ethers::types::{
    TxHash,
    U64,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::block::Height;

use super::ethereum::astria_withdrawer::{
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WithdrawalEvent {
    Sequencer(SequencerWithdrawalFilter),
    Ics20(Ics20WithdrawalFilter),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventWithMetadata {
    pub(crate) event: WithdrawalEvent,
    /// The block in which the log was emitted
    pub(crate) block_number: U64,
    /// The transaction hash in which the log was emitted
    pub(crate) transaction_hash: TxHash,
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
        memo: serde_json::to_vec(&memo).wrap_err("failed to serialize memo to json")?,
        fee_asset_id,
    };
    Ok(Action::BridgeUnlock(action))
}

#[derive(Debug, Serialize, Deserialize)]
struct Ics20WithdrawalMemo {
    memo: String,
    block_number: U64,
    transaction_hash: TxHash,
}

fn event_to_ics20_withdrawal(
    event: Ics20WithdrawalFilter,
    block_number: U64,
    transaction_hash: TxHash,
    fee_asset_id: asset::Id,
) -> eyre::Result<Action> {
    use ibc_types::core::client::Height as IbcHeight;

    let sender: [u8; 20] = event
        .sender
        .as_bytes()
        .try_into()
        .expect("U160 must be 20 bytes");
    let rollup_asset_denom = "transfer/channel-0/utia".to_string(); // TODO: this should be fetched from the sequencer
    let denom = Denom::from(rollup_asset_denom.clone());
    let (_, channel) = denom
        .prefix()
        .rsplit_once('/')
        .ok_or_eyre("denom must have a channel to be withdrawn via IBC")?;

    let memo = Ics20WithdrawalMemo {
        memo: String::from_utf8(event.memo.to_vec())
            .wrap_err("failed to convert event memo to utf8")?,
        block_number,
        transaction_hash,
    };

    let action = Ics20Withdrawal {
        denom: Denom::from(rollup_asset_denom),
        destination_chain_address: event.destination_chain_address,
        // note: this is actually a rollup address; we expect failed ics20 withdrawals to be
        // returned to the rollup.
        // this is only ok for now because addresses on the sequencer and the rollup are both 20
        // bytes, but this won't work otherwise.
        return_address: Address::from(sender),
        amount: event.amount.as_u128(),
        memo: serde_json::to_string(&memo).wrap_err("failed to serialize memo to json")?,
        fee_asset_id,
        // note: this refers to the timeout on the destination chain, which we are unaware of.
        // thus, we set it to the maximum possible value.
        timeout_height: IbcHeight::new(u64::MAX, u64::MAX)
            .wrap_err("failed to generate timeout height")?,
        timeout_time: u64::MAX,
        source_channel: channel
            .parse()
            .wrap_err("failed to parse channel from denom")?,
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
