use std::time::Duration;

use astria_core::{
    primitive::v1::{
        asset::{
            self,
            Denom,
        },
        Address,
        ASTRIA_ADDRESS_PREFIX,
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
use ibc_types::core::client::Height as IbcHeight;
use serde::{
    Deserialize,
    Serialize,
};

use crate::withdrawer::ethereum::astria_withdrawer_interface::{
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
    rollup_asset_denom: Denom,
    asset_withdrawal_divisor: u128,
) -> eyre::Result<Action> {
    let action = match event_with_metadata.event {
        WithdrawalEvent::Sequencer(event) => event_to_bridge_unlock(
            &event,
            event_with_metadata.block_number,
            event_with_metadata.transaction_hash,
            fee_asset_id,
            asset_withdrawal_divisor,
        )
        .wrap_err("failed to convert sequencer withdrawal event to action")?,
        WithdrawalEvent::Ics20(event) => event_to_ics20_withdrawal(
            event,
            event_with_metadata.block_number,
            event_with_metadata.transaction_hash,
            fee_asset_id,
            rollup_asset_denom,
            asset_withdrawal_divisor,
        )
        .wrap_err("failed to convert ics20 withdrawal event to action")?,
    };
    Ok(action)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BridgeUnlockMemo {
    pub(crate) block_number: U64,
    pub(crate) transaction_hash: TxHash,
}

fn event_to_bridge_unlock(
    event: &SequencerWithdrawalFilter,
    block_number: U64,
    transaction_hash: TxHash,
    fee_asset_id: asset::Id,
    asset_withdrawal_divisor: u128,
) -> eyre::Result<Action> {
    let memo = BridgeUnlockMemo {
        block_number,
        transaction_hash,
    };
    let action = BridgeUnlockAction {
        to: Address::builder()
            .array(event.destination_chain_address.to_fixed_bytes())
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .wrap_err("failed to construct destination address")?,
        amount: event
            .amount
            .as_u128()
            .checked_div(asset_withdrawal_divisor)
            .ok_or(eyre::eyre!(
                "failed to divide amount by asset withdrawal multiplier"
            ))?,
        memo: serde_json::to_vec(&memo).wrap_err("failed to serialize memo to json")?,
        fee_asset_id,
    };
    Ok(Action::BridgeUnlock(action))
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Ics20WithdrawalMemo {
    pub(crate) memo: String,
    pub(crate) block_number: U64,
    pub(crate) transaction_hash: TxHash,
}

fn event_to_ics20_withdrawal(
    event: Ics20WithdrawalFilter,
    block_number: U64,
    transaction_hash: TxHash,
    fee_asset_id: asset::Id,
    rollup_asset_denom: Denom,
    asset_withdrawal_divisor: u128,
) -> eyre::Result<Action> {
    // TODO: make this configurable
    const ICS20_WITHDRAWAL_TIMEOUT: Duration = Duration::from_secs(300);

    let sender = event.sender.to_fixed_bytes();
    let denom = rollup_asset_denom.clone();

    let (_, channel) = denom
        .prefix()
        .rsplit_once('/')
        .ok_or_eyre("denom must have a channel to be withdrawn via IBC")?;

    let memo = Ics20WithdrawalMemo {
        memo: event.memo,
        block_number,
        transaction_hash,
    };

    let action = Ics20Withdrawal {
        denom: rollup_asset_denom,
        destination_chain_address: event.destination_chain_address,
        // note: this is actually a rollup address; we expect failed ics20 withdrawals to be
        // returned to the rollup.
        // this is only ok for now because addresses on the sequencer and the rollup are both 20
        // bytes, but this won't work otherwise.
        return_address: Address::builder()
            .array(sender)
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .wrap_err("failed to construct return address")?,
        amount: event
            .amount
            .as_u128()
            .checked_div(asset_withdrawal_divisor)
            .ok_or(eyre::eyre!(
                "failed to divide amount by asset withdrawal multiplier"
            ))?,
        memo: serde_json::to_string(&memo).wrap_err("failed to serialize memo to json")?,
        fee_asset_id,
        // note: this refers to the timeout on the destination chain, which we are unaware of.
        // thus, we set it to the maximum possible value.
        timeout_height: IbcHeight::new(u64::MAX, u64::MAX)
            .wrap_err("failed to generate timeout height")?,
        timeout_time: calculate_packet_timeout_time(ICS20_WITHDRAWAL_TIMEOUT)
            .wrap_err("failed to calculate packet timeout time")?,
        source_channel: channel
            .parse()
            .wrap_err("failed to parse channel from denom")?,
    };
    Ok(Action::Ics20Withdrawal(action))
}

fn calculate_packet_timeout_time(timeout_delta: Duration) -> eyre::Result<u64> {
    tendermint::Time::now()
        .checked_add(timeout_delta)
        .ok_or_eyre("time must not overflow from now plus 10 minutes")?
        .unix_timestamp_nanos()
        .try_into()
        .wrap_err("failed to convert packet timeout i128 to u64")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::withdrawer::ethereum::astria_withdrawer_interface::SequencerWithdrawalFilter;

    #[test]
    fn event_to_bridge_unlock() {
        let denom = Denom::from("nria".to_string());
        let event_with_meta = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: [0u8; 20].into(),
                amount: 99.into(),
                destination_chain_address: [1u8; 20].into(),
            }),
            block_number: 1.into(),
            transaction_hash: [2u8; 32].into(),
        };
        let action = event_to_action(event_with_meta, denom.id(), denom.clone(), 1).unwrap();
        let Action::BridgeUnlock(action) = action else {
            panic!("expected BridgeUnlock action, got {action:?}");
        };

        let expected_action = BridgeUnlockAction {
            to: Address::builder()
                .array([1u8; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 99,
            memo: serde_json::to_vec(&BridgeUnlockMemo {
                block_number: 1.into(),
                transaction_hash: [2u8; 32].into(),
            })
            .unwrap(),
            fee_asset_id: denom.id(),
        };

        assert_eq!(action, expected_action);
    }

    #[test]
    fn event_to_bridge_unlock_divide_value() {
        let denom = Denom::from("nria".to_string());
        let event_with_meta = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: [0u8; 20].into(),
                amount: 990.into(),
                destination_chain_address: [1u8; 20].into(),
            }),
            block_number: 1.into(),
            transaction_hash: [2u8; 32].into(),
        };
        let divisor = 10;
        let action = event_to_action(event_with_meta, denom.id(), denom.clone(), divisor).unwrap();
        let Action::BridgeUnlock(action) = action else {
            panic!("expected BridgeUnlock action, got {action:?}");
        };

        let expected_action = BridgeUnlockAction {
            to: Address::builder()
                .array([1u8; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 99,
            memo: serde_json::to_vec(&BridgeUnlockMemo {
                block_number: 1.into(),
                transaction_hash: [2u8; 32].into(),
            })
            .unwrap(),
            fee_asset_id: denom.id(),
        };

        assert_eq!(action, expected_action);
    }

    #[test]
    fn event_to_ics20_withdrawal() {
        let denom = Denom::from("transfer/channel-0/utia".to_string());
        let destination_chain_address = "address".to_string();
        let event_with_meta = EventWithMetadata {
            event: WithdrawalEvent::Ics20(Ics20WithdrawalFilter {
                sender: [0u8; 20].into(),
                amount: 99.into(),
                destination_chain_address: destination_chain_address.clone(),
                memo: "hello".to_string(),
            }),
            block_number: 1.into(),
            transaction_hash: [2u8; 32].into(),
        };

        let action = event_to_action(event_with_meta, denom.id(), denom.clone(), 1).unwrap();
        let Action::Ics20Withdrawal(mut action) = action else {
            panic!("expected Ics20Withdrawal action, got {action:?}");
        };

        // TODO: instead of zeroing this, we should pass in the latest block time to the function
        // and generate the timeout time from that.
        action.timeout_time = 0; // zero this for testing

        let expected_action = Ics20Withdrawal {
            denom: denom.clone(),
            destination_chain_address,
            return_address: Address::builder()
                .array([0u8; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 99,
            memo: serde_json::to_string(&Ics20WithdrawalMemo {
                memo: "hello".to_string(),
                block_number: 1.into(),
                transaction_hash: [2u8; 32].into(),
            })
            .unwrap(),
            fee_asset_id: denom.id(),
            timeout_height: IbcHeight::new(u64::MAX, u64::MAX).unwrap(),
            timeout_time: 0, // zero this for testing
            source_channel: "channel-0".parse().unwrap(),
        };
        assert_eq!(action, expected_action);
    }
}
